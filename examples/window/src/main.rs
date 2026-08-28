use reveal_embedder_winit::WinitEmbedder;
use reveal_scheduler::Shell;

fn main() {
    WinitEmbedder::default().run(|platform| {
        Shell::new(platform, |_app| {
            // later: run_app(app, MyWidget);
        })
    });
}
