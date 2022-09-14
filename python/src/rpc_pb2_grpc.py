# Generated by the gRPC Python protocol compiler plugin. DO NOT EDIT!
"""Client and server classes corresponding to protobuf-defined services."""
import grpc

import rpc_pb2 as rpc__pb2


class RPCStub(object):
    """Missing associated documentation comment in .proto file."""

    def __init__(self, channel):
        """Constructor.

        Args:
            channel: A grpc.Channel.
        """
        self.TransactionFeed = channel.unary_stream(
                '/rpc.RPC/TransactionFeed',
                request_serializer=rpc__pb2.TransactionFeedRequest.SerializeToString,
                response_deserializer=rpc__pb2.SignedTransaction.FromString,
                )
        self.BlockFeed = channel.unary_stream(
                '/rpc.RPC/BlockFeed',
                request_serializer=rpc__pb2.BlockFeedRequest.SerializeToString,
                response_deserializer=rpc__pb2.SignedBlock.FromString,
                )
        self.Sync = channel.unary_unary(
                '/rpc.RPC/Sync',
                request_serializer=rpc__pb2.SyncRequest.SerializeToString,
                response_deserializer=rpc__pb2.BlockChain.FromString,
                )


class RPCServicer(object):
    """Missing associated documentation comment in .proto file."""

    def TransactionFeed(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def BlockFeed(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def Sync(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')


def add_RPCServicer_to_server(servicer, server):
    rpc_method_handlers = {
            'TransactionFeed': grpc.unary_stream_rpc_method_handler(
                    servicer.TransactionFeed,
                    request_deserializer=rpc__pb2.TransactionFeedRequest.FromString,
                    response_serializer=rpc__pb2.SignedTransaction.SerializeToString,
            ),
            'BlockFeed': grpc.unary_stream_rpc_method_handler(
                    servicer.BlockFeed,
                    request_deserializer=rpc__pb2.BlockFeedRequest.FromString,
                    response_serializer=rpc__pb2.SignedBlock.SerializeToString,
            ),
            'Sync': grpc.unary_unary_rpc_method_handler(
                    servicer.Sync,
                    request_deserializer=rpc__pb2.SyncRequest.FromString,
                    response_serializer=rpc__pb2.BlockChain.SerializeToString,
            ),
    }
    generic_handler = grpc.method_handlers_generic_handler(
            'rpc.RPC', rpc_method_handlers)
    server.add_generic_rpc_handlers((generic_handler,))


 # This class is part of an EXPERIMENTAL API.
class RPC(object):
    """Missing associated documentation comment in .proto file."""

    @staticmethod
    def TransactionFeed(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_stream(request, target, '/rpc.RPC/TransactionFeed',
            rpc__pb2.TransactionFeedRequest.SerializeToString,
            rpc__pb2.SignedTransaction.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def BlockFeed(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_stream(request, target, '/rpc.RPC/BlockFeed',
            rpc__pb2.BlockFeedRequest.SerializeToString,
            rpc__pb2.SignedBlock.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def Sync(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/rpc.RPC/Sync',
            rpc__pb2.SyncRequest.SerializeToString,
            rpc__pb2.BlockChain.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)