# proto-chain

Primitive blockchain implementation in python and rust

## Run nodes
You can choose to run either python or rust node. For example
```
cd rust; make run node=0
cd python; make run node=1 peers="[0]"
```
If parameter peers is specified, new node will sync the blockchain state with the specified list of peers, otherwise new blockchain will be created

## API
This requires python node to be running
You can use rest api by acccessing swagger on http://0.0.0.0:8000/docs or http://0.0.0.0:8001/docs

Ports can be configured in config.yaml file

## Dev client
You can also use python client to call rest and rpc methods on the node

### Send coins from the miner wallet to another:
```
make client method=transact
```
### Send coins from the miner wallet to the second wallet and then immediately to the third:
```
make client method=sequence
```
### Attempt double spend:
```
make client method=double_spend
```
### Pull all blocks:
```
make client method=sync
```
### Stream new blocks:
```
make client method=block_feed
```
### Stream new transactions:
```
make client method=transaction_feed
```

