from concurrent import futures
from queue import Queue, Empty

import grpc

from proto_chain import rpc_pb2, rpc_pb2_grpc


class RPC(rpc_pb2_grpc.RPCServicer):
    def __init__(self, runtime, transaction_queues, block_queues, stop_event):
        super().__init__()
        self._runtime = runtime
        self._transaction_queues = transaction_queues
        self._block_queues = block_queues
        self._stop_event = stop_event

    def TransactionFeed(self, request, context):
        queue = Queue()
        self._transaction_queues.append(queue)
        while not self._stop_event.is_set():
            try:
                yield queue.get(timeout=1).pb
            except Empty:
                ...

    def BlockFeed(self, request, context):
        queue = Queue()
        self._block_queues.append(queue)
        while not self._stop_event.is_set():
            try:
                yield queue.get(timeout=1).pb
            except Empty:
                ...

    def Sync(self, request, context):
        self._runtime.add_peer(request.address)
        return rpc_pb2.BlockChain(
            blocks=[b.pb for b in self._runtime.blockchain.blocks]
        )


def get_server(port, runtime, transaction_queue, block_queue, stop_event):
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    rpc_pb2_grpc.add_RPCServicer_to_server(RPC(runtime, transaction_queue, block_queue, stop_event), server)
    server.add_insecure_port(f'[::]:{port}')
    return server
