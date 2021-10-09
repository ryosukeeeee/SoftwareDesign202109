# SoftwareDesign202109

Software Design 2021年9月号のRustでわかるメモリ管理のサンプルコード

[Software Design 2021年9月号｜技術評論社](https://gihyo.jp/magazine/SD/archive/2021/202109)
## 第４章

### `/examples/tera.rs`

テンプレートエンジンを追加

実行方法

```
$ cargo run --example tera &
$ curl -dname=ryosukeeeeee http://localhost:3000
Hello, ryosukeeeeee!
```


### `examples/db.rs`

データベースとの連携機能を追加

実行方法

```
$ cargo run --example db
$ curl -dtitle=test -d'content=test2 input' http://localhost:3000/posts
27daa5b9-0e7a-4853-b1d3-942e206289a3
$ curl http://localhost:3000/posts/27daa5b9-0e7a-4853-b1d3-942e206289a3
id: 27daa5b9-0e7a-4853-b1d3-942e206289a3
title: test
content:
test2 input
```

