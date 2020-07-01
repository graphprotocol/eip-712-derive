use eip_712_derive::*;

#[derive(Default)]
struct Transaction {
    from: Person,
    to: Person,
    tx: Asset,
}

impl StructType for Transaction {
    const TYPE_NAME: &'static str = "Transaction";
    fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
        visitor.visit("from", &self.from);
        visitor.visit("to", &self.to);
        visitor.visit("tx", &self.tx);
    }
}

#[derive(Default)]
struct Person {
    wallet: Address,
    name: String,
}
impl StructType for Person {
    const TYPE_NAME: &'static str = "Person";
    fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
        visitor.visit("wallet", &self.wallet);
        visitor.visit("name", &self.name);
    }
}

#[derive(Default)]
struct Asset {
    token: Address,
    amount: U256,
}

impl StructType for Asset {
    const TYPE_NAME: &'static str = "Asset";
    fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
        visitor.visit("token", &self.token);
        visitor.visit("amount", &self.amount);
    }
}

#[test]
fn encode_transaction_type() {
    let expected = "Transaction(Person from,Person to,Asset tx)Asset(address token,uint256 amount)Person(address wallet,string name)";

    let value: Transaction = Default::default();
    assert_eq!(encode_type(&value), expected);
}
