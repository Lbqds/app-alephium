use crate::{
    error_code::ErrorCode,
    ledger_sdk_stub::nbgl_review::NbglStreamingReview,
    ui::nbgl::{nbgl_review_warning, new_nbgl_review},
};
use ledger_device_sdk::nbgl::{Field, NbglReviewStatus, TransactionType};

pub struct TxReviewerInner {
    pub review_started: bool,
    pub display_settings: bool,
    is_tx_execute_script: bool,
    reviewer: Option<NbglStreamingReview>,
}

impl TxReviewerInner {
    pub fn new() -> TxReviewerInner {
        TxReviewerInner {
            review_started: false,
            display_settings: false,
            is_tx_execute_script: false,
            reviewer: None,
        }
    }

    #[inline]
    fn get_reviewer(&self) -> &NbglStreamingReview {
        assert!(self.reviewer.is_some());
        self.reviewer.as_ref().unwrap()
    }

    pub fn set_display_settings(&mut self, display_settings: bool) {
        self.display_settings = display_settings;
    }

    pub fn set_tx_execute_script(&mut self, is_tx_execute_script: bool) {
        assert!(self.reviewer.is_none());
        self.is_tx_execute_script = is_tx_execute_script;
        self.reviewer = Some(new_nbgl_review(
            TransactionType::Transaction,
            is_tx_execute_script,
        ));
    }

    // Start review tx details
    pub fn start_review(&mut self) -> Result<(), ErrorCode> {
        let message = if self.is_tx_execute_script {
            "Review transaction"
        } else {
            "Review transaction to send assets"
        };
        if self.get_reviewer().start(message, "") {
            self.review_started = true;
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }

    pub fn review_fields<'a>(
        &self,
        fields: &'a [Field<'a>],
        _message: &str,
    ) -> Result<(), ErrorCode> {
        if self.get_reviewer().continue_review(fields) {
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }

    // Review transfer that sends to self
    pub fn review_self_transfer(&mut self, fee_field: &Field) -> Result<(), ErrorCode> {
        let fee_field = Field {
            name: fee_field.name,
            value: fee_field.value,
        };
        if self.is_tx_execute_script {
            self.finish_review(&[fee_field])
        } else {
            let fields = &[
                Field {
                    name: "Amount",
                    value: "Self-transfer",
                },
                fee_field,
            ];
            self.finish_review(fields)
        }
    }

    // Review the warning for external inputs, i.e. inputs that are not from the device address
    pub fn warning_external_inputs(&self) -> Result<(), ErrorCode> {
        let approved = nbgl_review_warning(
            "External inputs",
            "This transaction has inputs from addresses not associated with this device.",
            "Continue",
            "Reject",
        );
        if approved {
            Ok(())
        } else {
            Err(ErrorCode::UserCancelled)
        }
    }

    pub fn finish_review<'a>(&mut self, fee_fields: &'a [Field<'a>]) -> Result<(), ErrorCode> {
        assert!(!fee_fields.is_empty());
        self.display_settings = false;
        self.review_fields(fee_fields, "Fees")?;
        let message = if self.is_tx_execute_script {
            "Accept risk and sign transaction"
        } else {
            "Sign transaction to send assets"
        };
        if self.get_reviewer().finish(message) {
            NbglReviewStatus::new().show(true);
            Ok(())
        } else {
            NbglReviewStatus::new().show(false);
            Err(ErrorCode::UserCancelled)
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        // Since `reset` is called when blind signing checks fails,
        // we cannot reset the `display_settings` within the reset function.
        // Instead, we will reset the `display_settings` in the `finish_review` function.
        self.review_started = false;
        self.reviewer = None;
        self.is_tx_execute_script = false;
    }

    #[inline]
    pub fn output_index_as_field(&self) -> bool {
        true
    }
}
