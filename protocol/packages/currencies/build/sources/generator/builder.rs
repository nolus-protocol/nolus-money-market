use super::{super::CurrentModule, Generator, StaticContext};

#[derive(Clone, Copy)]
pub(in super::super) struct Builder<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
>(
    &'static_context StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
);

impl<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
>
    Builder<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
{
    #[inline]
    pub const fn new(
        static_context: &'static_context StaticContext<
            'protocol,
            'host_currency,
            'dex_currencies,
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
    ) -> Self {
        Self(static_context)
    }

    #[inline]
    pub const fn lease(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
    > {
        self.build(CurrentModule::Lease)
    }

    #[inline]
    pub const fn lpn(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        false,
    > {
        self.build(CurrentModule::Lpn)
    }

    #[inline]
    pub const fn native(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
    > {
        self.build(CurrentModule::Native)
    }

    #[inline]
    pub const fn payment_only(
        &self,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        true,
    > {
        self.build(CurrentModule::PaymentOnly)
    }

    #[inline]
    const fn build<const PAIRS_GROUP: bool>(
        &self,
        current_module: CurrentModule,
    ) -> Generator<
        'static_context,
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        PAIRS_GROUP,
    > {
        let Self(static_context) = *self;

        Generator {
            static_context,
            current_module,
        }
    }
}
