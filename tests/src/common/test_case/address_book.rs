use platform::contract::CodeId;
use sdk::cosmwasm_std::Addr;

pub(crate) struct AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
    dispatcher_addr: Dispatcher,
    treasury_addr: Treasury,
    profit_addr: Profit,
    leaser_addr: Leaser,
    lpp_addr: Lpp,
    oracle_addr: Oracle,
    time_alarms_addr: TimeAlarms,
    lease_code_id: CodeId,
}

impl AddressBook<(), (), (), (), (), (), ()> {
    pub(super) const fn new(lease_code_id: CodeId) -> Self {
        Self {
            dispatcher_addr: (),
            treasury_addr: (),
            profit_addr: (),
            leaser_addr: (),
            lpp_addr: (),
            oracle_addr: (),
            time_alarms_addr: (),
            lease_code_id,
        }
    }
}

impl<Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<(), Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_dispatcher(
        self,
        dispatcher_addr: Addr,
    ) -> AddressBook<Addr, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Addr, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn dispatcher(&self) -> &Addr {
        &self.dispatcher_addr
    }
}

impl<Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, (), Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_treasury(
        self,
        treasury_addr: Addr,
    ) -> AddressBook<Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn treasury(&self) -> &Addr {
        &self.treasury_addr
    }
}

impl<Dispatcher, Treasury, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, (), Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_profit(
        self,
        profit_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Addr, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Addr, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn profit(&self) -> &Addr {
        &self.profit_addr
    }
}

impl<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, (), Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_leaser(
        self,
        leaser_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>
{
    pub const fn leaser(&self) -> &Addr {
        &self.leaser_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, (), Oracle, TimeAlarms>
{
    pub(super) fn with_lpp(
        self,
        lpp_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms>
{
    pub const fn lpp(&self) -> &Addr {
        &self.lpp_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, (), TimeAlarms>
{
    pub(super) fn with_oracle(
        self,
        oracle_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>
{
    pub const fn oracle(&self) -> &Addr {
        &self.oracle_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, ()>
{
    pub(super) fn with_time_alarms(
        self,
        time_alarms_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>
{
    pub const fn time_alarms(&self) -> &Addr {
        &self.time_alarms_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn lease_code_id(&self) -> CodeId {
        self.lease_code_id
    }
}
