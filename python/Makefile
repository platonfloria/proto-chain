run:
	poetry run python src/proto_chain/node.py $(node) $(peers)

proto:
	poetry run python -m grpc_tools.protoc -I.. --python_out=./src/proto_chain --grpc_python_out=./src/proto_chain ../rpc.proto
