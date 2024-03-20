use platform::contract::Code;
use sdk::cosmwasm_std::Addr;

pub(crate) struct AddressBook<
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
> {
    protocols_registry: ProtocolsRegistry,
    dispatcher_addr: Dispatcher,
    treasury_addr: Treasury,
    profit_addr: Profit,
    profit_ica_addr: Profit,
    reserve: Reserve,
    leaser_addr: Leaser,
    lpp_addr: Lpp,
    oracle_addr: Oracle,
    time_alarms_addr: TimeAlarms,
    lease_code: Code,
}

impl AddressBook<(), (), (), (), (), (), (), (), ()> {
    pub(super) const fn new(lease_code: Code) -> Self {
        Self {
            protocols_registry: (),
            dispatcher_addr: (),
            treasury_addr: (),
            profit_addr: (),
            profit_ica_addr: (),
            reserve: (),
            leaser_addr: (),
            lpp_addr: (),
            oracle_addr: (),
            time_alarms_addr: (),
            lease_code,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<(), Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_protocols_registry(
        self,
        protocols_registry_addr: Addr,
    ) -> AddressBook<Addr, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    {
        AddressBook {
            protocols_registry: protocols_registry_addr,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Addr, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn protocols_registry(&self) -> &Addr {
        &self.protocols_registry
    }
}

impl<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<ProtocolsRegistry, (), Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_dispatcher(
        self,
        dispatcher_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Addr,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<ProtocolsRegistry, Addr, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn dispatcher(&self) -> &Addr {
        &self.dispatcher_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<ProtocolsRegistry, Dispatcher, (), Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub(super) fn with_treasury(
        self,
        treasury_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Addr,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Addr,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn treasury(&self) -> &Addr {
        &self.treasury_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        (),
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub(super) fn with_profit(
        self,
        profit_addr: Addr,
        profit_ica_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Addr,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr,
            profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Addr,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn profit(&self) -> &Addr {
        &self.profit_addr
    }

    pub const fn profit_ica(&self) -> &Addr {
        &self.profit_ica_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        (),
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub(super) fn with_reserve(
        self,
        reserve: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Addr,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Addr,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn reserve(&self) -> &Addr {
        &self.reserve
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        (),
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub(super) fn with_leaser(
        self,
        leaser_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn leaser(&self) -> &Addr {
        &self.leaser_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        (),
        Oracle,
        TimeAlarms,
    >
{
    pub(super) fn with_lpp(
        self,
        lpp_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Addr,
        Oracle,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Addr,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn lpp(&self) -> &Addr {
        &self.lpp_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        (),
        TimeAlarms,
    >
{
    pub(super) fn with_oracle(
        self,
        oracle_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >
{
    pub const fn oracle(&self) -> &Addr {
        &self.oracle_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>
    AddressBook<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, ()>
{
    pub(super) fn with_time_alarms(
        self,
        time_alarms_addr: Addr,
    ) -> AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    > {
        AddressBook {
            protocols_registry: self.protocols_registry,
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            profit_ica_addr: self.profit_ica_addr,
            reserve: self.reserve,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr,
            lease_code: self.lease_code,
        }
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>
    AddressBook<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, Addr>
{
    pub const fn time_alarms(&self) -> &Addr {
        &self.time_alarms_addr
    }
}

impl<ProtocolsRegistry, Dispatcher, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >
{
    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }
}
