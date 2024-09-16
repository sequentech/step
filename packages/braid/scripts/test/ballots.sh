cargo run --release --bin demo_tool -- post-ballots --port=49153 --password=postgres --board-count $1 --ciphertexts $2
echo $2 >> stats.txt