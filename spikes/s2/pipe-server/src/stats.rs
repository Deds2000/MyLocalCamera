// Estatísticas do medidor do spike S2 (60 s): quadros, perdas, vazão e
// latência escrita->leitura (p50/p95) a partir dos acks do cliente.

#[derive(Default)]
pub struct Stats {
    pub frames_sent: u64,
    pub bytes_sent: u64,
    pub frames_acked: u64,
    pub frames_lost: u64, // tentados no prazo mas não enviados (atraso/backpressure)
    pub late: u64,        // round-trips que estouraram o tick de 30 fps
    pub connections: u64,
    pub write_errors: u64,
    pub read_errors: u64,
    pub latencies_us: Vec<u64>,
}

impl Stats {
    pub fn record_latency(&mut self, us: u64) {
        self.latencies_us.push(us);
    }

    fn percentile(&self, pct: usize) -> Option<u64> {
        if self.latencies_us.is_empty() {
            return None;
        }
        let mut v = self.latencies_us.clone();
        v.sort_unstable();
        let idx = ((v.len() as u64 * pct as u64 + 99) / 100) as usize;
        v.get(idx.saturating_sub(1).min(v.len() - 1)).copied()
    }

    pub fn json(&self, channel: &str, duration_s: f64) -> String {
        let mbps = if duration_s > 0.0 {
            self.bytes_sent as f64 / 1e6 / duration_s
        } else {
            0.0
        };
        let p50 = self.percentile(50).unwrap_or(0);
        let p95 = self.percentile(95).unwrap_or(0);
        let max = self
            .latencies_us
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        format!(
            "{{\n  \"channel\": \"{channel}\",\n  \"duration_s\": {},\n  \"frames_sent\": {},\n  \"bytes_sent\": {},\n  \"throughput_mbps\": {:.1},\n  \"frames_acked\": {},\n  \"frames_lost\": {},\n  \"late_roundtrips\": {},\n  \"connections\": {},\n  \"write_errors\": {},\n  \"read_errors\": {},\n  \"latency_us\": {{ \"p50\": {}, \"p95\": {}, \"max\": {} }}\n}}",
            duration_s,
            self.frames_sent,
            self.bytes_sent,
            mbps,
            self.frames_acked,
            self.frames_lost,
            self.late,
            self.connections,
            self.write_errors,
            self.read_errors,
            p50,
            p95,
            max,
        )
    }

    pub fn summary_line(&self, duration_s: f64) -> String {
        format!(
            "enviados={} perdidos={} acks={} conexoes={} latencia p50/p95={}us/{}us vazao={:.1} MB/s",
            self.frames_sent,
            self.frames_lost,
            self.frames_acked,
            self.connections,
            self.percentile(50).unwrap_or(0),
            self.percentile(95).unwrap_or(0),
            self.bytes_sent as f64 / 1e6 / duration_s.max(0.001),
        )
    }
}
