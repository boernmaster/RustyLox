//! Prometheus text format exporter

use crate::collector::{AppMetrics, SystemMetrics};

/// Exports metrics in Prometheus text exposition format
pub struct PrometheusExporter;

impl PrometheusExporter {
    /// Generate Prometheus text format output
    pub fn export(system: &SystemMetrics, app: &AppMetrics, uptime_seconds: u64) -> String {
        let mut output = String::new();

        // Daemon uptime
        output.push_str("# HELP rustylox_uptime_seconds Daemon uptime in seconds\n");
        output.push_str("# TYPE rustylox_uptime_seconds counter\n");
        output.push_str(&format!("rustylox_uptime_seconds {}\n\n", uptime_seconds));

        // System uptime
        output.push_str("# HELP rustylox_system_uptime_seconds System uptime in seconds\n");
        output.push_str("# TYPE rustylox_system_uptime_seconds counter\n");
        output.push_str(&format!(
            "rustylox_system_uptime_seconds {}\n\n",
            system.uptime_seconds
        ));

        // CPU usage
        output.push_str("# HELP rustylox_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE rustylox_cpu_usage_percent gauge\n");
        output.push_str(&format!(
            "rustylox_cpu_usage_percent {:.2}\n\n",
            system.cpu_usage_percent
        ));

        // CPU count
        output.push_str("# HELP rustylox_cpu_count Number of CPUs\n");
        output.push_str("# TYPE rustylox_cpu_count gauge\n");
        output.push_str(&format!("rustylox_cpu_count {}\n\n", system.cpu_count));

        // Memory metrics
        output.push_str("# HELP rustylox_memory_total_bytes Total physical memory\n");
        output.push_str("# TYPE rustylox_memory_total_bytes gauge\n");
        output.push_str(&format!(
            "rustylox_memory_total_bytes {}\n\n",
            system.memory_total_bytes
        ));

        output.push_str("# HELP rustylox_memory_used_bytes Used physical memory\n");
        output.push_str("# TYPE rustylox_memory_used_bytes gauge\n");
        output.push_str(&format!(
            "rustylox_memory_used_bytes {}\n\n",
            system.memory_used_bytes
        ));

        output.push_str("# HELP rustylox_memory_usage_percent Memory usage percentage\n");
        output.push_str("# TYPE rustylox_memory_usage_percent gauge\n");
        output.push_str(&format!(
            "rustylox_memory_usage_percent {:.2}\n\n",
            system.memory_usage_percent
        ));

        // Disk metrics
        output.push_str("# HELP rustylox_disk_total_bytes Total disk space\n");
        output.push_str("# TYPE rustylox_disk_total_bytes gauge\n");
        output.push_str(&format!(
            "rustylox_disk_total_bytes {}\n\n",
            system.disk_total_bytes
        ));

        output.push_str("# HELP rustylox_disk_used_bytes Used disk space\n");
        output.push_str("# TYPE rustylox_disk_used_bytes gauge\n");
        output.push_str(&format!(
            "rustylox_disk_used_bytes {}\n\n",
            system.disk_used_bytes
        ));

        output.push_str("# HELP rustylox_disk_usage_percent Disk usage percentage\n");
        output.push_str("# TYPE rustylox_disk_usage_percent gauge\n");
        output.push_str(&format!(
            "rustylox_disk_usage_percent {:.2}\n\n",
            system.disk_usage_percent
        ));

        // Load average
        output.push_str("# HELP rustylox_load_average System load average (1m, 5m, 15m windows)\n");
        output.push_str("# TYPE rustylox_load_average gauge\n");
        output.push_str(&format!(
            "rustylox_load_average{{window=\"1m\"}} {:.4}\n",
            system.load_average[0]
        ));
        output.push_str(&format!(
            "rustylox_load_average{{window=\"5m\"}} {:.4}\n",
            system.load_average[1]
        ));
        output.push_str(&format!(
            "rustylox_load_average{{window=\"15m\"}} {:.4}\n\n",
            system.load_average[2]
        ));

        // MQTT metrics
        output.push_str("# HELP rustylox_mqtt_messages_total Total MQTT messages\n");
        output.push_str("# TYPE rustylox_mqtt_messages_total counter\n");
        output.push_str(&format!(
            "rustylox_mqtt_messages_total{{direction=\"received\"}} {}\n",
            app.mqtt_messages_received
        ));
        output.push_str(&format!(
            "rustylox_mqtt_messages_total{{direction=\"sent\"}} {}\n\n",
            app.mqtt_messages_sent
        ));

        // HTTP metrics
        output.push_str("# HELP rustylox_http_requests_total Total HTTP requests\n");
        output.push_str("# TYPE rustylox_http_requests_total counter\n");
        output.push_str(&format!(
            "rustylox_http_requests_total {}\n\n",
            app.http_requests_total
        ));

        output.push_str("# HELP rustylox_http_requests_errors_total Total HTTP error responses\n");
        output.push_str("# TYPE rustylox_http_requests_errors_total counter\n");
        output.push_str(&format!(
            "rustylox_http_requests_errors_total {}\n\n",
            app.http_requests_errors
        ));

        // System info labels
        output.push_str("# HELP rustylox_info System information\n");
        output.push_str("# TYPE rustylox_info gauge\n");
        output.push_str(&format!(
            "rustylox_info{{hostname=\"{}\",os_name=\"{}\",os_version=\"{}\"}} 1\n",
            system.hostname, system.os_name, system.os_version
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{AppMetrics, SystemMetrics};
    use chrono::Utc;

    fn sample_system_metrics() -> SystemMetrics {
        SystemMetrics {
            cpu_usage_percent: 25.5,
            memory_total_bytes: 8_000_000_000,
            memory_used_bytes: 2_000_000_000,
            memory_usage_percent: 25.0,
            disk_total_bytes: 100_000_000_000,
            disk_used_bytes: 40_000_000_000,
            disk_usage_percent: 40.0,
            uptime_seconds: 3600,
            hostname: "testhost".to_string(),
            os_name: "Linux".to_string(),
            os_version: "6.0".to_string(),
            kernel_version: "6.0.0".to_string(),
            cpu_count: 4,
            load_average: [0.5, 0.4, 0.3],
            collected_at: Utc::now(),
        }
    }

    #[test]
    fn test_export_contains_key_metrics() {
        let system = sample_system_metrics();
        let app = AppMetrics {
            mqtt_messages_received: 100,
            mqtt_messages_sent: 50,
            http_requests_total: 1000,
            http_requests_errors: 5,
        };
        let output = PrometheusExporter::export(&system, &app, 3600);

        assert!(output.contains("rustylox_cpu_usage_percent 25.50"));
        assert!(output.contains("rustylox_memory_usage_percent 25.00"));
        assert!(output.contains("rustylox_uptime_seconds 3600"));
        assert!(output.contains("direction=\"received\"} 100"));
        assert!(output.contains("direction=\"sent\"} 50"));
        assert!(output.contains("rustylox_http_requests_total 1000"));
    }
}
