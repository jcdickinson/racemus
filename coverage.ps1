$Env:CARGO_INCREMENTAL=0
$Env:RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads"
cargo test
grcov ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./target/debug/coverage --excl_line "#\[derive\(" --excl_br_line "#\[derive\(" --excl_start "mod tests \{" --excl_br_start "mod tests \{"
Start-Process ./target/debug/coverage/index.html
