run_node:
	poetry run python src/node.py $(node) $(peers)

run_client:
	poetry run python src/client.py $(method) $(data)

proto:
	python -m grpc_tools.protoc -I.. --python_out=./src --grpc_python_out=./src ../rpc.proto
