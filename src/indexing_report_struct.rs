use crate::r#trait::filter_report::ReportItemType;
use crate::r#type::types::ResponseError;

#[derive(Debug, Clone, Copy)]
pub struct IndexingReport {
    pub index_commission: Option<usize>,
    pub index_commission_sys: Option<usize>,
    pub index_commission_bank: Option<usize>,
    pub index_commission_payment: Option<usize>,
    pub index_commission_eops: Option<usize>,
    pub index_commission_partner: Option<usize>,
    pub commission_secondbank: Option<usize>,
    pub index_date: Option<usize>,
    pub index_provider: Option<usize>,
    pub index_provider_id: Option<usize>,
    pub index_mode: Option<usize>,
    pub index_status: Option<usize>,
    pub index_amount: Option<usize>,
    pub index_vendor: Option<usize>,
    pub index_merchant_id: Option<usize>,
    pub index_tran_type: Option<usize>,
    pub index_payment_system: Option<usize>
}

impl IndexingReport {
    pub fn new() -> Self {
        Self {
            index_commission: None,
            index_commission_sys: None,
            index_commission_bank: None,
            index_commission_payment: None,
            index_commission_eops: None,
            index_commission_partner: None,
            index_date: None,
            index_provider: None,
            index_mode: None,
            index_status: None,
            commission_secondbank: None,
            index_amount: None,
            index_vendor: None,
            index_provider_id: None,
            index_tran_type: None,
            index_merchant_id: None,
            index_payment_system: None,
        }
    }

    /// Поиск индекса по заголовкам
    pub fn find_index_by_name(&mut self, record: Vec<&str>, report_type: ReportItemType) {
        for (index, field) in record.into_iter().enumerate() {
            // Поля которые есть в [Платежах] и [Переводов]
            match field.to_lowercase().as_str() {
                "провайдер" => self.index_provider = Some(index),
                "provider_id" => self.index_provider_id = Some(index),
                "статус" => self.index_status = Some(index),
                "режим" => self.index_mode = Some(index),
                "сумма" => self.index_amount = Some(index),
                "комиссия" => self.index_commission = Some(index),
                "комиссия eops" | "commission_eops" => {
                    self.index_commission_eops = Some(index)
                }
                "комиссия COMANYNAME" | "commission_COMANYNAME" => {
                    self.index_commission_sys = Some(index)
                }
                "комиссия bank" | "commission_bank" => {
                    self.index_commission_bank = Some(index)
                }
                "комиссия partner" | "commission_partner" => {
                    self.index_commission_partner = Some(index)
                }
                "дата транзакции" => self.index_date = Some(index),
                _ => {}
            }
            match report_type {
                // Поля которые есть только в [Переводах]
                ReportItemType::Remittance => match field.to_lowercase().as_str() {
                    "tran_type" => self.index_tran_type = Some(index),
                    _ => {}
                },
                // Поля которые есть только в [Платежах]
                ReportItemType::Payments => match field.to_lowercase().as_str() {
                    "комиссия payment" => self.index_commission_payment = Some(index),
                    "commission_secondbank" => self.commission_secondbank = Some(index),
                    "вендор" => self.index_vendor = Some(index),
                    "вендор id" => self.index_merchant_id = Some(index),
                    "платёжная система" => self.index_payment_system = Some(index),
                    _ => {},
                },
                ReportItemType::Unknown => {}
                ReportItemType::Empty => {}
                ReportItemType::Null => {}
            }
        }
    }

    // Проверка на обязательные поля
    pub fn check_which_fields_not_found(
        &self,
        report_type: ReportItemType,
    ) -> Result<(), ResponseError> {
        // =========================================================================================== \\
        if self.index_commission.is_none() {
            return Err((423134, "index_commission field is None".to_string()));
        }
        if self.index_date.is_none() {
            return Err((8423141, "index_date field is None".to_string()));
        }
        if self.index_provider.is_none() {
            return Err((423142, "index_Provider field is None".to_string()));
        }
        if self.index_provider_id.is_none() {
            return Err((423143, "index_provider_id field is None".to_string()));
        }
        if self.index_mode.is_none() {
            return Err((423144, "index_mode field is None".to_string()));
        }
        if self.index_status.is_none() {
            return Err((423145, "index_status field is None".to_string()));
        }
        if self.index_amount.is_none() {
            return Err((423146, "index_amount field is None".to_string()));
        }
        if self.index_commission_sys.is_none() {
            return Err((423135, "index_commission_sys field is None".to_string()));
        }
        if self.index_commission_bank.is_none() {
            return Err((
                423136,
                "index_commission_bank field is None".to_string(),
            ));
        }
        if self.index_commission_eops.is_none() {
            return Err((423138, "index_commission_eops field is None".to_string()));
        }
        if self.index_commission_partner.is_none() {
            return Err((423139, "index_commission_partner field is None".to_string()));
        }
        // =========================================================================================== \\

        match report_type {
            ReportItemType::Remittance => {
                if self.index_tran_type.is_none() {
                    return Err((423147, "index_commission_payment field is None".to_string()));
                }
            }
            ReportItemType::Payments => {
                if self.index_commission_payment.is_none() {
                    return Err((423137, "index_commission_payment field is None".to_string()));
                }
                if self.index_vendor.is_none() {
                    return Err((423147, "index_Merchantfield is None".to_string()));
                }
            }
            ReportItemType::Unknown => {}
            ReportItemType::Empty => {}
            ReportItemType::Null => {}
        }

        Ok(())
    }
}
