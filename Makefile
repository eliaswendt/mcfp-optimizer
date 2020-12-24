sample:
	cargo run --release sample_data/ && dot -Tpdf graph.dot > graph.pdf

real:
	cargo run --release real_data/