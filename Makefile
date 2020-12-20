run_sample:
	cargo run --release sample_data/ && dot -Tpdf graph.dot > graph.pdf

run_real:
	cargo run --release real_data/