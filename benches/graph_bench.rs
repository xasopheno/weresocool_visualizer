use criterion::{criterion_group, criterion_main, Criterion};
use weresocool_visualizer::core::WereSoCoolSpectrumConfig;
use weresocool_visualizer::graph_handler::GraphHandler;
use weresocool_visualizer::window_handler::WindowHandler;
use winit::event_loop::EventLoop;

fn benchmark_update_and_draw(c: &mut Criterion) {
    let mut group = c.benchmark_group("GraphHandler");
    let event_loop = EventLoop::new();

    let config = WereSoCoolSpectrumConfig::new();
    let window_handler =
        WindowHandler::new(config.logical_width, config.logical_height, &event_loop);

    let mut graph_handler = GraphHandler::new(
        config.width as usize,
        config.height as usize,
        config.visual_buffer_size as usize,
        config.visual_buffer_size / config.fft_div as usize,
        &window_handler.window,
    )
    .unwrap();

    group.bench_function("update_and_draw", |b| {
        b.iter(|| graph_handler.update_and_draw())
    });

    group.finish();
}

criterion_group!(benches, benchmark_update_and_draw);
criterion_main!(benches);
