pub enum Route {
    Page,
    Bundle,
    Stylesheet,
    StatusJson,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub enum Error {
    NotFound,
    MethodNotAllowed,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub fn parse<'x, I>(method: &str, path: &str, headers: I) -> Result<Route, Error>
    where I: Iterator<Item = (&'x str, &'x [u8])>,
{
    if method != "GET" {
        return Err(Error::MethodNotAllowed);
    }
    let path = match path.find(|c| c == '?' || c == '#') {
        Some(idx) => &path[..idx],
        None => path
    };
    if path.ends_with("/bundle.js") {
        return Ok(Route::Bundle);
    } else if path.ends_with("/main.css") {
        return Ok(Route::Stylesheet);
    } else if path.ends_with("/status.json") {
        return Ok(Route::StatusJson);
    } else {
        return Ok(Route::Page);
    }
}
