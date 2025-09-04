use std::marker::PhantomData;

use currency::{AnyVisitor, CurrencyDTO, CurrencyDef, Group, MemberOf};
use dex::{SwapOutputTask, SwapTask, WithOutputTask};

pub struct WithOutCurrency<SwapTask, OutTaskFry, Cmd> {
    swap_task: SwapTask,
    cmd: Cmd,
    _out_task_fry: PhantomData<OutTaskFry>,
}

pub trait OutTaskFactory<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    fn new_task<OutC>(swap_task: SwapTaskT) -> impl SwapOutputTask<SwapTaskT, OutC = OutC>
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<SwapTaskT::OutG> + MemberOf<<SwapTaskT::InG as Group>::TopG>;
}

impl<SwapTask, OutTaskFry, Cmd> WithOutCurrency<SwapTask, OutTaskFry, Cmd> {
    pub fn from(swap_task: SwapTask, cmd: Cmd) -> Self {
        Self {
            swap_task,
            cmd,
            _out_task_fry: PhantomData,
        }
    }
}
impl<SwapTaskT, OutTaskFry, Cmd> AnyVisitor<SwapTaskT::OutG>
    for WithOutCurrency<SwapTaskT, OutTaskFry, Cmd>
where
    SwapTaskT: SwapTask,
    OutTaskFry: OutTaskFactory<SwapTaskT>,
    Cmd: WithOutputTask<SwapTaskT>,
{
    type Outcome = Cmd::Output;

    fn on<C>(self, _def: &CurrencyDTO<C::Group>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<<SwapTaskT::OutG as Group>::TopG> + MemberOf<SwapTaskT::OutG>,
    {
        self.cmd.on(OutTaskFry::new_task::<C>(self.swap_task))
    }
}
