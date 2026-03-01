wasm_release_dir := "target/wasm32-unknown-unknown/wasm-release"

@build_untitled_game_wasm:
    cargo build --package untitled_game --bin untitled_game --profile wasm-release --target wasm32-unknown-unknown
    wasm-opt -Os --output {{wasm_release_dir}}/untitled_game_optimized.wasm {{wasm_release_dir}}/untitled_game.wasm

@build_untitled_game:
    cargo build --package untitled_game --bin untitled_game --release

# When using wasm-server-runner we need to be in the correct dir for assets to load
[working-directory: 'untitled_game']
@run_untitled_game_wasm_dev:
    cargo run --bin untitled_game --target wasm32-unknown-unknown

# When using wasm-server-runner we need to be in the correct dir for assets to load
[working-directory: 'untitled_game']
@run_untitled_game_dev:
    cargo run --bin untitled_game

[working-directory: 'untitled_game']
@build_glb:
    Blender -b art/untitled_game.blend --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='assets/untitled_game.glb', export_extras=True)"
