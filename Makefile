sample:
	cargo run --release sample_data/ && dot -Tpdf graphs/graph.dot > graphs/graph.pdf

real:
	cargo run --release real_data/

graphs:
	dot -Tpdf graphs/*.dot > *.pdf