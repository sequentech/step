// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_gauge,
    CounterVec, HistogramVec, IntGauge,
};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use std::time::Instant;

const METRIC_HTTP_REQUESTS_TOTAL: &str = "harvest_http_requests_total";
const METRIC_HTTP_REQUEST_DURATION_SECONDS: &str =
    "harvest_http_request_duration_seconds";
const METRIC_HTTP_REQUESTS_IN_FLIGHT: &str = "harvest_http_requests_in_flight";

/// Label used for requests that don't match any registered Rocket route
/// (e.g. 404s or pre-routing failures).
const UNKNOWN_ROUTE: &str = "unknown";

lazy_static! {
    static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        METRIC_HTTP_REQUESTS_TOTAL,
        "Total number of HTTP requests handled by harvest",
        &["method", "route", "status"]
    )
    .expect("Failed to register harvest_http_requests_total metric");
    static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec =
        register_histogram_vec!(
            METRIC_HTTP_REQUEST_DURATION_SECONDS,
            "HTTP request duration in seconds",
            &["method", "route"],
            vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                30.0
            ]
        )
        .expect(
            "Failed to register harvest_http_request_duration_seconds metric"
        );
    static ref HTTP_REQUESTS_IN_FLIGHT: IntGauge = register_int_gauge!(
        METRIC_HTTP_REQUESTS_IN_FLIGHT,
        "Number of HTTP requests currently being handled"
    )
    .expect("Failed to register harvest_http_requests_in_flight metric");
}

struct RequestStart(Instant);

/// Force all metric statics to initialize and register with the prometheus
/// global registry. Call once at startup so metrics appear in `/metrics`
/// before the first HTTP request arrives.
pub fn init_metrics() {
    let _ = &*HTTP_REQUESTS_TOTAL;
    let _ = &*HTTP_REQUEST_DURATION_SECONDS;
    let _ = &*HTTP_REQUESTS_IN_FLIGHT;
}

pub struct MetricsFairing;

#[rocket::async_trait]
impl Fairing for MetricsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Prometheus Metrics",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        HTTP_REQUESTS_IN_FLIGHT.inc();
        req.local_cache(|| RequestStart(Instant::now()));
    }

    async fn on_response<'r>(
        &self,
        req: &'r Request<'_>,
        res: &mut Response<'r>,
    ) {
        let start = req.local_cache(|| RequestStart(Instant::now()));
        let duration = start.0.elapsed().as_secs_f64();
        let method = req.method().as_str();
        // Use the matched route pattern (e.g. "/election/<tenant_id>") rather
        // than the actual URI to keep label cardinality bounded.
        let route = req
            .route()
            .map(|r| r.uri.to_string())
            .unwrap_or_else(|| UNKNOWN_ROUTE.to_string());
        let status = res.status().code.to_string();

        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, &route, &status])
            .inc();
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&[method, &route])
            .observe(duration);
        HTTP_REQUESTS_IN_FLIGHT.dec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::core::Collector;

    #[test]
    fn test_metrics_are_registered() {
        let _ = &*HTTP_REQUESTS_TOTAL;
        let _ = &*HTTP_REQUEST_DURATION_SECONDS;
        let _ = &*HTTP_REQUESTS_IN_FLIGHT;

        assert!(!HTTP_REQUESTS_TOTAL.desc().is_empty());
        assert!(!HTTP_REQUEST_DURATION_SECONDS.desc().is_empty());
        assert_eq!(HTTP_REQUESTS_IN_FLIGHT.get(), 0);
    }

    #[test]
    fn test_counter_increments() {
        let _ = &*HTTP_REQUESTS_TOTAL;
        let before = HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test", "200"])
            .get();
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test", "200"])
            .inc();
        let after = HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test", "200"])
            .get();
        assert_eq!(after - before, 1.0);
    }
}
