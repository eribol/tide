#[allow(dead_code)]
pub async fn find_port() -> async_std::net::SocketAddr {
    async_std::net::TcpListener::bind("localhost:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
}

#[allow(dead_code)]
pub fn headers<'a>(
    headers: impl IntoIterator<
        Item = (
            &'a http_types::headers::HeaderName,
            &'a http_types::headers::HeaderValues,
        ),
    >,
    header: &str,
) -> Option<Vec<&'a str>> {
    let header_name: http_types::headers::HeaderName = header.parse().unwrap();

    headers
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>()
        .get(&header_name)
        .map(|header_values| {
            header_values
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
        })
}
