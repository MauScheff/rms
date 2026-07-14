#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewRoute {
    Index,
    App,
    Snapshot,
    Health,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParsedViewRequest {
    Get(ViewRoute),
    Head(ViewRoute),
}

impl ParsedViewRequest {
    pub(crate) fn route(self) -> ViewRoute {
        match self {
            Self::Get(route) | Self::Head(route) => route,
        }
    }

    pub(crate) fn head_only(self) -> bool {
        matches!(self, Self::Head(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewRequestRejection {
    MethodNotAllowed,
    RouteNotFound,
}

impl ViewRequestRejection {
    pub(crate) fn status(self) -> u16 {
        match self {
            Self::MethodNotAllowed => 405,
            Self::RouteNotFound => 404,
        }
    }

    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::MethodNotAllowed => "method not allowed",
            Self::RouteNotFound => "route not found",
        }
    }
}

pub fn parse_view_request(
    method: &str,
    request_target: &str,
) -> Result<ParsedViewRequest, ViewRequestRejection> {
    let head_only = match method {
        "GET" => false,
        "HEAD" => true,
        _ => return Err(ViewRequestRejection::MethodNotAllowed),
    };
    let path = request_target.split('?').next().unwrap_or(request_target);
    let route = match path {
        "/" | "/index.html" => ViewRoute::Index,
        "/app.js" => ViewRoute::App,
        "/api/snapshot" => ViewRoute::Snapshot,
        "/api/health" => ViewRoute::Health,
        _ => return Err(ViewRequestRejection::RouteNotFound),
    };

    if head_only {
        Ok(ParsedViewRequest::Head(route))
    } else {
        Ok(ParsedViewRequest::Get(route))
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub struct ViewRouteCase {
    method: &'static str,
    target: &'static str,
    accepted: bool,
}

#[cfg(test)]
pub fn generate_view_route_cases() -> Vec<ViewRouteCase> {
    const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
    const TARGETS: [&str; 6] = [
        "/",
        "/index.html",
        "/app.js",
        "/api/snapshot",
        "/api/health",
        "/unknown",
    ];
    METHODS
        .into_iter()
        .flat_map(|method| {
            TARGETS.into_iter().map(move |target| ViewRouteCase {
                method,
                target,
                accepted: matches!(method, "GET" | "HEAD") && target != "/unknown",
            })
        })
        .collect()
}

#[cfg(test)]
#[test]
pub fn property_system_viewer_routes_are_read_only() {
    let cases = generate_view_route_cases();
    assert_eq!(cases.len(), 42);
    for case in cases {
        let result = parse_view_request(case.method, case.target);
        assert_eq!(
            result.is_ok(),
            case.accepted,
            "unexpected route result for {} {}",
            case.method,
            case.target
        );
        if let Ok(parsed) = result {
            assert!(matches!(case.method, "GET" | "HEAD"));
            assert_eq!(parsed.head_only(), case.method == "HEAD");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_strings_do_not_expand_the_route_table() {
        let request = parse_view_request("GET", "/api/snapshot?cache=ignored").unwrap();
        assert_eq!(request.route(), ViewRoute::Snapshot);
    }

    #[test]
    fn mutation_methods_are_rejected_before_route_classification() {
        let rejection = parse_view_request("POST", "/unknown").unwrap_err();
        assert_eq!(rejection, ViewRequestRejection::MethodNotAllowed);
    }
}
