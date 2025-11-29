# This is: https://github.com/bddap/watch-stl-rust
watch-stl out.stl &

echo model.yascad | entr cargo run -- -i model.yascad -o out.stl
