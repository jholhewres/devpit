use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::{pump, Flow, Out, Sink};

/// A page that acknowledges, counting what reached it.
struct Counted(Arc<AtomicUsize>);

impl Sink for Counted {
    fn send(&self, _: Out, _: Vec<u8>) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst);
        true
    }
    fn acks(&self) -> bool {
        true
    }
}

/// Past [`Flow::MOST`] unacknowledged the pump stops taking frames, so the
/// bounded channel fills and the pty is not read; an ack lets it go on.
#[test]
fn the_pump_holds_past_the_window_until_the_page_acknowledges() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let half = Flow::MOST / 2;
        let (send, mut frames) = tokio::sync::mpsc::channel(16);
        for _ in 0..8 {
            send.send(vec![b'y'; half]).await.expect("queued");
        }
        let flow = Arc::new(Flow::default());
        let got = Arc::new(AtomicUsize::new(0));
        let pumping = {
            let (flow, sink) = (flow.clone(), Counted(got.clone()));
            tokio::spawn(async move { pump(&mut frames, &flow, &sink).await })
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(got.load(Ordering::SeqCst), 3, "held once past the window");
        flow.ack(2 * half);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            got.load(Ordering::SeqCst),
            5,
            "two more for what was acknowledged"
        );
        pumping.abort();
    });
}
