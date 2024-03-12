# acnetworkprotocol

Things to document

High-level:

- Encapsulation (fragments)
- Reliability over UDP
- Cryptography/checksums/ISAAC cipher stuff
- Flow, wtf is it

Packet structure

- Packets have a TransitHeader
- The TransitHeader has a Size field which is the size after the TransitHeader
  - To calculate this, you need to write the data first and then record its size
