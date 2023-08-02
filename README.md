This project provides an API to pin a raw IPFS block with Pinata.

The official [Pinata API](https://docs.pinata.cloud/reference/pinningpinfiletoipfs) unfortunately cannot do this, which is why this workaround is needed. This program accepts HTTP `post` requests to `/put_block`. The body of the request is interpreted as the raw bytes of an IPFS block. The block is sent to an IPFS node through the `/api/v0/block/put` route in the [RPC API](https://docs.ipfs.tech/reference/kubo/rpc/). Then resulting CID is pinned with Pinata. The resulting CID is returned in the body of the response.
