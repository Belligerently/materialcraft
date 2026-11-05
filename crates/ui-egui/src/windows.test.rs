#[cfg(test)]
mod import_tests {
    use crate::{MaterialCraftApp, state::UiState};
    use materialcraft_engine::formats::{Depth, encode};
    use materialcraft_engine::image::{Image, Kind};

    #[test]
    fn importing_a_bitmap_file_is_one_undo_entry() {
        let png = encode(&Image::filled(4, 4, Kind::Color, [0.5, 0.2, 0.1, 1.0]), "png", Depth::U8, true).unwrap();
        let path = std::env::temp_dir().join(format!("mc-import-{}.png", std::process::id()));
        std::fs::write(&path, png).unwrap();
        let mut app = MaterialCraftApp::headless(materialcraft_engine::sample_session(), UiState::default());
        let base = app.session.history().len();
        crate::graph_view::import_bitmap_file(&mut app, &path.display().to_string());
        std::fs::remove_file(&path).ok();
        assert_eq!(app.session.history().len(), base + 1, "history: {:?}", app.session.history());
        assert_eq!(app.session.history().last().map(String::as_str), Some("Import bitmap"));
        app.session.undo();
        assert_eq!(app.session.history().len(), base);
    }
}
