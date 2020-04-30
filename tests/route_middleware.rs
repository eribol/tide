mod test_utils;
use futures::future::BoxFuture;
use http_service_mock::make_server;
use http_types::{headers::HeaderName, Method, Request};
use std::convert::TryInto;
use test_utils::headers;
use tide::Middleware;

#[derive(Debug)]
struct TestMiddleware(HeaderName, &'static str);

impl TestMiddleware {
    fn with_header_name(name: &'static str, value: &'static str) -> Self {
        Self(name.try_into().unwrap(), value)
    }
}

impl<State: Send + Sync + 'static> Middleware<State> for TestMiddleware {
    fn handle<'a>(
        &'a self,
        req: tide::Request<State>,
        next: tide::Next<'a, State>,
    ) -> BoxFuture<'a, tide::Result<tide::Response>> {
        Box::pin(async move {
            let res = next.run(req).await?;
            Ok(res.set_header(self.0.clone(), self.1))
        })
    }
}

async fn echo_path<State>(req: tide::Request<State>) -> tide::Result<String> {
    Ok(req.uri().path().to_string())
}

#[test]
fn route_middleware() {
    let mut app = tide::new();
    let mut foo_route = app.at("/foo");
    foo_route // /foo
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    foo_route
        .at("/bar") // nested, /foo/bar
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .get(echo_path);
    foo_route // /foo
        .post(echo_path)
        .reset_middleware()
        .put(echo_path);
    let mut server = make_server(app).unwrap();

    let req = Request::new(Method::Get, "http://localhost/foo".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Foo"), Some(vec!["foo"]));

    let req = Request::new(Method::Post, "http://localhost/foo".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Foo"), Some(vec!["foo"]));

    let req = Request::new(Method::Put, "http://localhost/foo".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Foo"), None);

    let req = Request::new(Method::Get, "http://localhost/foo/bar".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Foo"), Some(vec!["foo"]));
    assert_eq!(headers(&res, "X-Bar"), Some(vec!["bar"]));
}

#[test]
fn app_and_route_middleware() {
    let mut app = tide::new();
    app.middleware(TestMiddleware::with_header_name("X-Root", "root"));
    app.at("/foo")
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    app.at("/bar")
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .get(echo_path);
    let mut server = make_server(app).unwrap();

    let req = Request::new(Method::Get, "http://localhost/foo".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Root"), Some(vec!["root"]));
    assert_eq!(headers(&res, "X-Foo"), Some(vec!["foo"]));
    assert_eq!(headers(&res, "X-Bar"), None);

    let req = Request::new(Method::Get, "http://localhost/bar".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Root"), Some(vec!["root"]));
    assert_eq!(headers(&res, "X-Foo"), None);
    assert_eq!(headers(&res, "X-Bar"), Some(vec!["bar"]));
}

#[test]
fn nested_app_with_route_middleware() {
    let mut inner = tide::new();
    inner.middleware(TestMiddleware::with_header_name("X-Inner", "inner"));
    inner
        .at("/baz")
        .middleware(TestMiddleware::with_header_name("X-Baz", "baz"))
        .get(echo_path);

    let mut app = tide::new();
    app.middleware(TestMiddleware::with_header_name("X-Root", "root"));
    app.at("/foo")
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    app.at("/bar")
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .nest(inner);
    let mut server = make_server(app).unwrap();

    let req = Request::new(Method::Get, "http://localhost/foo".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Root"), Some(vec!["root"]));
    assert_eq!(headers(&res, "X-Inner"), None);
    assert_eq!(headers(&res, "X-Foo"), Some(vec!["foo"]));
    assert_eq!(headers(&res, "X-Bar"), None);
    assert_eq!(headers(&res, "X-Baz"), None);

    let req = Request::new(Method::Get, "http://localhost/bar/baz".parse().unwrap());
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Root"), Some(vec!["root"]));
    assert_eq!(headers(&res, "X-Inner"), Some(vec!["inner"]));
    assert_eq!(headers(&res, "X-Foo"), None);
    assert_eq!(headers(&res, "X-Bar"), Some(vec!["bar"]));
    assert_eq!(headers(&res, "X-Baz"), Some(vec!["baz"]));
}

#[test]
fn subroute_not_nested() {
    let mut app = tide::new();
    app.at("/parent") // /parent
        .middleware(TestMiddleware::with_header_name("X-Parent", "Parent"))
        .get(echo_path);
    app.at("/parent/child") // /parent/child, not nested
        .middleware(TestMiddleware::with_header_name("X-Child", "child"))
        .get(echo_path);
    let mut server = make_server(app).unwrap();

    let req = Request::new(
        Method::Get,
        "http://localhost/parent/child".parse().unwrap(),
    );
    let res = server.simulate(req).unwrap();
    assert_eq!(headers(&res, "X-Parent"), None);
    assert_eq!(headers(&res, "X-Child"), Some(vec!["child"]));
}
