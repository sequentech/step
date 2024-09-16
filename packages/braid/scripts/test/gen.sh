# hard coded for threshold = 2, trustees = 3
cargo run --release --bin demo_tool -- gen-configs --port=49153 --password=postgrespw --num-trustees=3 --threshold=2