use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Request, Response, Server, StatusCode};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::sync::Arc;
use std::{convert::Infallible, net::SocketAddr};
use tera::{Context, Tera};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Deserialize)]
struct NewPost<'a> {
    title: &'a str,
    content: &'a str,
}

struct Post {
    id: Uuid,
    title: String,
    content: String,
}

impl Post {
    // テンプレートを使って投稿を文字列にレンダリング
    fn render(&self, tera: Arc<Tera>) -> String {
        let mut ctx = Context::new();
        ctx.insert("id", &self.id);
        ctx.insert("title", &self.title);
        ctx.insert("content", &self.content);
        tera.render("post_template", &ctx).unwrap()
    }
}

// リクエストパスの/posts/(post_id)からpost_idの部分を取り出す関数
fn get_id(req: &Request<Body>) -> Uuid {
    let path = req.uri().path();
    let post_id = &path[7..];
    Uuid::parse_str(post_id).unwrap()
}

async fn find_post(
    req: Request<Body>,
    tera: Arc<Tera>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    let id = get_id(&req);

    let post = conn
        .lock()
        .await
        .query_row(
            "SELECT id, title, content FROM posts WHERE id = ?1",
            params![id],
            |row| {
                Ok(Post {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                })
            },
        )
        .optional()
        .unwrap();

    match post {
        Some(post) => Ok(Response::new(post.render(tera).into())),
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

async fn create_post(
    req: Request<Body>,
    _: Arc<Tera>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    let body = hyper::body::to_bytes(req.into_body()).await?;
    // フォームデータを取り出す
    let new_post = serde_urlencoded::from_bytes::<NewPost>(&body).unwrap();
    // DBに登録するためにUUIDも生成しておく
    let id = Uuid::new_v4();

    conn.lock()
        .await
        .execute(
            "INSERT INTO posts(id, title, content) VALUES (?1, ?2, ?3)",
            params![&id, new_post.title, new_post.content],
        )
        .unwrap();
    Ok(Response::new(id.to_string().into()))
}

static TEMPLATE: &str = "Hello, {{name}}!";
static POST_TEMPLATE: &str = "id: {{id}}\ntitle: {{title}}\ncontent:\n{{content}}\n";

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World!".into()))
}

async fn route(
    req: Request<Body>,
    tera: Arc<Tera>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    match (req.uri().path(), req.method().as_str()) {
        ("/", "GET") => handle_with_body(req, tera).await,
        ("/", _) => handle(req).await.map_err(|e| match e {}),
        ("/posts", "POST") => create_post(req, tera, conn).await,
        (path, "GET") if path.starts_with("/posts/") => find_post(req, tera, conn).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

async fn handle_with_body(req: Request<Body>, tera: Arc<Tera>) -> Result<Response<Body>, Error> {
    let body = hyper::body::to_bytes(req.into_body()).await?;

    let body = std::str::from_utf8(&body).unwrap();
    let name = body.strip_prefix("name=").unwrap();

    let mut ctx = Context::new();
    ctx.insert("name", name);
    let rendered = tera.render("hello", &ctx).unwrap();

    Ok(Response::new(rendered.into()))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let mut tera = Tera::default();
    tera.add_raw_template("hello", TEMPLATE).unwrap();
    tera.add_raw_template("post_template", POST_TEMPLATE)
        .unwrap();

    let tera = Arc::new(tera);

    let conn = Connection::open_in_memory().unwrap();
    let conn = Arc::new(Mutex::new(conn));

    // テーブルを作成しておく
    conn.lock()
        .await
        .execute(
            "CREATE TABLE posts (
            id      BLOB PRIMARY KEY,
            title   TEXT NOT NULL,
            content TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

    let make_svc = make_service_fn(|_conn| {
        let tera = tera.clone();
        let conn = conn.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                route(req, tera.clone(), conn.clone())
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
