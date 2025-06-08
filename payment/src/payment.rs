pub struct PaymentGateway {
    pub authorized_orders: HashMap<u64, OrderDTO>,
    pub communicators: HashMap<SocketAddr, Communicator>,
}
