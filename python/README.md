# ToyCoin

## Run nodes
Run the following commands in different terminals
```
make run_node node=0
make run_node node=1 peers="[0]"
```

## API
You can use rest api by acccessing swagger on http://0.0.0.0:8000/docs or http://0.0.0.0:8001/docs

Ports can be configured in config.yaml file

## Dev client
You can also use python client to call rest and rpc methods on the node

### Send coins from the miner wallet to another:
```
make run_client method=transact
```
### Send coins from the miner wallet to the second wallet and then immediately to the third:
```
make run_client method=sequence
```
### Attempt double spend:
```
make run_client method=double_spend
```
### Pull all blocks:
```
make run_client method=sync
```
### Stream new blocks:
```
make run_client method=block_feed
```
### Stream new transactions:
```
make run_client method=transaction_feed
```
