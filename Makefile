run_with_graph:
	cargo run --release && dot -Tpdf graph.dot > graph.pdf