@0xcdbd9ec1c7981634;

struct Message {
    payload @0 :Data;
    nonce @1 :Int64;
    expirationTime @2 :Int64;
}

interface Reconcile {
    hashes @0 () -> (hashes :List(Data));
    query @1 (hash :Data) -> (message :Message);
    requestReconciliation @2 (hashes :List(Data));
}