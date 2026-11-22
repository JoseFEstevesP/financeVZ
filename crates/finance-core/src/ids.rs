macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub fn new(value: i64) -> Self {
                Self(value)
            }

            pub fn get(self) -> i64 {
                self.0
            }
        }
    };
}

id_newtype!(ExpenseId);
id_newtype!(IncomeId);
id_newtype!(CategoryId);
id_newtype!(RecurringPaymentId);
