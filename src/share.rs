pub mod share_helper;

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize, Deserializer};

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};

use dotenv_codegen::dotenv;

use tokio::sync::RwLock as TokioRwLock;

use crate::r#trait::filter_report::{Filter, ReportType, ReportItemType, Status};

use crate::indexing_report_struct::IndexingReport;
use crate::r#type::types::{ChunksInReport, RecordStrings, ReportsDateRange, ResponseError};
use tracing::error;
use crate::helper::build_tasks::processing_tasks;
use crate::helper::create_file::create_fs::create_file_name;

use crate::helper::generate_xlsx::create_task;


#[derive(Debug)]
pub struct Share {
    pub reports: Reports,
    generated_now: Arc<Mutex<u16>>,
    max_count_record_in_reports: Arc<Mutex<u16>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// [Report type] Тип отчета
    /// к примеру TaxiCompony(Таксопарки), Agent(Агент), Merchant(Мерчант)
    report_type: ReportType,
    /// [Report Provider] Содержит имя провайдера для которого генерируется отчет
    report_organization_name: String,
    /// [Report organization id]
    report_organization_id: String,
    /// [Report items] Содержит себе элименты отчета, к примеру наш отчет генерируется для таксопарка
    /// в частности такой отчет будет содержать два элемента это подсчеты по платежам и переводам
    /// то есть Payments и Remittance
    report_items: HashMap<ReportItemType, ReportItem>,
    /// [Is report read] Проверяем был и прочитана информация для отчета полностью
    pub(crate) is_report_read: bool,
    /// [create at] Время создания репорта
    /// Report creation time
    pub create_at: i64,
}

#[derive(Debug, Clone)]
pub struct ArcMutexWrapper<T>(pub(crate) Arc<TokioRwLock<T>>);

impl <T>ArcMutexWrapper <T> {
    pub fn new_arc_mutex_wrapper(report: Arc<TokioRwLock<T>>) -> ArcMutexWrapper<T> { ArcMutexWrapper(report) }

    pub async fn get_mutex(&self) -> &Arc<TokioRwLock<T>> { &self.0 }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ArcMutexWrapper<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let inner = T::deserialize(deserializer)?;
        Ok(ArcMutexWrapper(Arc::new(TokioRwLock::new(inner))))
    }
}

// impl<T: Serialize> Serialize for ArcMutexWrapper<T> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportItem {
    /// [Filter] Фильтры по которым был собран этот отчет
    pub filter: Filter,
    /// [Amount] Общаяя сумма
    pub amount: f64,
    /// [Days amount] сумма забитая по числу на все дни
    pub days_amount: Vec<(String, f64)>,
    /// [Days length transactions] количество транзакций за определенный день
    pub days_len_transaction: Vec<(String, u64)>,
    /// [length transactions] Количество транзакций за весь период отчета
    pub len_transactions: u128,
    /// [commission by day] Коммиссия по дням
    pub commission_by_day: Vec<(String, f64)>,
    /// [commission] общаяя коммиссия
    pub commission: f64,
    /// [percentage of workload] Процент загруженности отчета
    pub percent_load: f64,
    /// [id of the having chunk] Id имеющих чанков
    pub id_having_chunk: Vec<u32>,
    /// [summary by Provider] Тут содержутся данные по каждому провайдеру
    /// summary_by_Provider.0 = Имя Вендора = Вендор,
    /// summary_by_Provider.1 = Количество транзакций вендора = количество транзакций текущего вендора с определенными филтрами
    /// summary_by_Provider.2 = Общая Сумма вендора = Сумма,
    /// summary_by_Provider.3 = Сумма комиссии с Поставщика = Комиссия,
    /// summary_by_Provider.4 = Вознаграждение Банка 0,2% = Комиссия AloqBank
    /// summary_by_Provider.5 = Вознаграждение COMANYNAME = Комиссия COMANYNAME
    pub summary_by_Provider: Vec<(String, u32, f64, f64, f64, f64)>,
    /// [general report on payments] Тут содержатся данные только для отчета [TaxiCompany]
    /// general_report_on_payments_taxi_company.0 = Имя Provider
    /// general_report_on_payments_taxi_company.1 = Количество транзакций текущего VENDOR(Provider)
    /// general_report_on_payments_taxi_company.2 = Сумма(amount) без комиссий текущего VENDOR(Provider)
    /// general_report_on_payments_taxi_company.3 = Вознаграждение COMANYNAME(commission) VENDOR(Provider)
    pub general_report_on_payments_taxi_company: Vec<(String, u128, f64, f64)>,
    /// [general report on payments] Тут содержатся данные только для отчета [TaxiCompany]
    /// general_report_on_payments_merchant.0 = Имя VENDOR(Provider) Вендора(провайдера)
    /// general_report_on_payments_merchant.1 = Имя платежной системы
    /// general_report_on_payments_merchant.2 = Количество транзакций текущего VENDOR(Provider)
    /// general_report_on_payments_merchant.3 = Сумма(amount) без комиссий текущего VENDOR(Provider)
    /// general_report_on_payments_merchant.4 = Вознаграждение COMANYNAME(commission) VENDOR(Provider)
    pub general_report_on_payments_merchant: Vec<(String, String, u128, f64, f64)>,
    /// [general report on payments agent] Тут содержутся данные только для отчета [Agent]
    /// general_report_on_payments_agent.0 = Имя провайдера(VENDOR)
    /// general_report_on_payments_agent.1 = Количество транзакций текушего провайдера
    /// general_report_on_payments_agent.2 = Сумма без комиссий текущего провайдера
    /// general_report_on_payments_agent.3 = Комиссиия текущего провайдера
    /// general_report_on_payments_agent.4 = Возногрождение COMANYNAME COMANYNAME + bank
    /// general_report_on_payments_agent.5 = Возногрождение Агента
    pub general_report_on_remittance_agent: Vec<(String, u32, f64, f64, f64, f64)>,
    /// [refill amount] Общаяя сумма с пополнение счета
    pub refill_amount: f64,
    /// [days_in_report] дни которые есть в отчете
    pub days_in_report: HashSet<String>,
    /// [all_types_of_commissions] Все виды комиссий включая общию коммиссию
    pub all_types_of_commissions: CommissionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionType {
    /// Поля под названием "Коммиссия" в листе "Платежи"
    pub commission: f64,
    /// Поля под названием "Комиссия COMANYNAME" в листе "Платежи"
    pub commission_pay_sys: f64,
    /// Поля под названием "Комиссия Bank" в листе "Платежи"
    pub commission_bank: f64,
    /// Поля под названием "Комиссия Payment" в листе "Платежи"
    pub commission_payment: f64,
    /// Поля под названием "Комиссия EOPS" в листе "Платежи"
    pub commission_eops: f64,
    /// Поля под названием "Комиссия Partner" в листе "Платежи"
    pub commission_partner: f64,
}

impl ReportItem {
    pub fn new(filter: &Filter) -> ReportItem {
        ReportItem {
            filter: filter.clone(),
            amount: 0.0,
            days_amount: vec![],
            days_len_transaction: vec![],
            len_transactions: 0,
            commission_by_day: vec![],
            commission: 0.0,
            percent_load: 0.0,
            id_having_chunk: vec![],
            summary_by_Provider: vec![],
            general_report_on_payments_taxi_company: vec![],
            general_report_on_payments_merchant: vec![],
            general_report_on_remittance_agent: vec![],
            refill_amount: 0.0,
            days_in_report: HashSet::new(),
            all_types_of_commissions: CommissionType {
                commission: 0.0,
                commission_pay_sys: 0.0,
                commission_bank: 0.0,
                commission_payment: 0.0,
                commission_eops: 0.0,
                commission_partner: 0.0,
            },
        }
    }

    pub fn calculate_all_type_commissions(
        &mut self,
        records: &Vec<Vec<String>>,
        collect_indexing: &IndexingReport,
        type_report: &ReportItemType,
    ) {
        let mut commission = 0.0;
        let mut commission_pay_sys = 0.0;
        let mut commission_bank = 0.0;
        let mut commission_payment = 0.0;
        let mut commission_eops = 0.0;
        let mut commission_partner = 0.0;

        // Перебераем все виды комисий
        records.iter().for_each(|record| {
            // Сохраняем число комиссий содержащиеся в чанке в переменную
            commission += record[collect_indexing.index_commission.unwrap()]
                .parse::<f64>()
                .unwrap();
            commission_pay_sys += record[collect_indexing.index_commission_sys.unwrap()]
                .parse::<f64>()
                .unwrap();
            commission_bank += record[collect_indexing.index_commission_bank.unwrap()]
                .parse::<f64>()
                .unwrap();
            commission_eops += record[collect_indexing.index_commission_eops.unwrap()]
                .parse::<f64>()
                .unwrap();
            commission_partner += record[collect_indexing.index_commission_partner.unwrap()]
                .parse::<f64>()
                .unwrap();
            if type_report == &ReportItemType::Payments {
                commission_payment += record[collect_indexing.index_commission_payment.unwrap()]
                    .parse::<f64>()
                    .unwrap();
            }
        });

        // записываем получившуюся комиссию из чанка в report
        self.all_types_of_commissions.commission += commission;
        self.all_types_of_commissions.commission_pay_sys += commission_pay_sys;
        self.all_types_of_commissions.commission_bank += commission_bank;
        self.all_types_of_commissions.commission_eops += commission_eops;
        self.all_types_of_commissions.commission_partner += commission_partner;
        if type_report == &ReportItemType::Payments {
            self.all_types_of_commissions.commission_payment += commission_payment;
        }
    }

    pub fn calculate_commission(
        &mut self,
        record: &Vec<Vec<String>>,
        collecting_indexing: &IndexingReport,
    ) {
        record.iter().for_each(|commission| {
            self.commission += commission[collecting_indexing.index_commission.unwrap()]
                .parse::<f64>()
                .unwrap_or(0.0);
        })
    }

    /// Добовляем в поля [days_amount] дату и сумму переведенную за эту дату
    pub fn set_days_amount(&mut self, day_amount: Vec<(String, f64)>) {
        day_amount.into_iter().for_each(|amount| {
            let mut exist_day = false;
            for date_amount in &mut self.days_amount {
                if date_amount.0 == amount.0 {
                    date_amount.1 += amount.1;
                    exist_day = true;
                }
            }

            if !exist_day {
                self.days_amount.push(amount);
            }
        })
    }

    /// Добовляем в поля [commission_by_day] дату и сумму переведенную за эту дату
    pub fn set_days_commissions(&mut self, day_commission: Vec<(String, f64)>) {
        day_commission.into_iter().for_each(|amount| {
            let mut exist_day = false;
            for date_commission in &mut self.commission_by_day {
                if date_commission.0 == amount.0.to_string() {
                    date_commission.1 += amount.1;
                    exist_day = true;
                }
            }

            if !exist_day {
                self.commission_by_day.push(amount);
            }
        })
    }

    pub fn set_len_transactions(&mut self, records_len: usize) {
        self.len_transactions += records_len as u128
    }

    pub fn get_percent_load(&self) -> f64 {
        self.percent_load
    }

    // @43252
    pub fn build_summary_by_Provider(
        &mut self,
        chunk: &Vec<Vec<String>>,
        collect_indexing: &IndexingReport,
        type_report: &ReportItemType,
    ) -> Result<(), ResponseError> {
        let index_tran_type_or_merchant= collect_indexing
            .index_tran_type
            .unwrap_or(collect_indexing.index_vendor.unwrap_or(0));

        for record in chunk {
            let mut is_exist_Merchant= false;
            if self.summary_by_Provider.len() == 0 {
                match type_report {
                    ReportItemType::Remittance | ReportItemType::Payments => {
                        if index_tran_type_or_merchant == 0 {
                            return Err((
                                5432520,
                                "Индекс index_tran_type не был найден!".to_string(),
                            ));
                        }

                        self.summary_by_Provider.push((
                            // Поставщик
                            record[index_tran_type_or_merchant].to_string(),
                            // кол-во
                            1,
                            // Сумма
                            record[collect_indexing.index_amount.unwrap()]
                                .parse::<f64>()
                                .unwrap_or(0.0),
                            // Сумма комиссии с Поставщика
                            record[collect_indexing.index_commission.unwrap()]
                                .parse::<f64>()
                                .unwrap_or(0.0),
                            // Вознаграждение Банка 0,2%
                            record[collect_indexing.index_commission_bank.unwrap()]
                                .parse::<f64>()
                                .unwrap_or(0.0),
                            // Вознаграждение COMANYNAME
                            record[collect_indexing.index_commission_sys.unwrap()]
                                .parse::<f64>()
                                .unwrap_or(0.0),
                        ));
                    }
                    ReportItemType::Unknown => {
                        return Err((
                            5432521,
                            "Тип отчета \"Unknown\" не был корректно обработан".to_string(),
                        ))
                    }
                    ReportItemType::Empty => {
                        return Err((
                            5432522,
                            "Тип отчета \"Empty\" не был корректно обработан".to_string(),
                        ))
                    }
                    ReportItemType::Null => {
                        return Err((
                            5432523,
                            "Тип отчета \"Null\" не был корректно обработан".to_string(),
                        ))
                    }
                }

                continue;
            }

            for vendor in self.summary_by_Provider.iter_mut() {
                // Считаем сумму, комиссию под каждого вендора
                if vendor.0 == record[index_tran_type_or_merchant] {
                    vendor.1 += 1;
                    vendor.2 += record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    vendor.3 += record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    vendor.4 += record[collect_indexing.index_commission_bank.unwrap()]
                            .parse::<f64>()
                            .unwrap_or(0.0);
                    vendor.5 += record[collect_indexing.index_commission_sys.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    is_exist_Merchant= true;
                    continue;
                }
            }

            if !is_exist_Merchant{
                self.summary_by_Provider.push((
                    // Называние вендора
                    record[collect_indexing
                        .index_vendor
                        .unwrap_or(index_tran_type_or_merchant)]
                    .to_string(),
                    // Количество транзакций
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    // Общаяя коммиссия
                    record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap(),
                    // Коммиссия AloqBank
                    record[collect_indexing.index_commission_bank.unwrap()]
                        .parse::<f64>()
                        .unwrap(),
                    // Коммиссия COMANYNAME
                    record[collect_indexing.index_commission_sys.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                ));
            }
        }

        Ok(())
    }

    pub fn build_general_report_taxi_company(
        &mut self,
        chunk: &Vec<RecordStrings>,
        collect_indexing: &IndexingReport,
    ) -> Result<(), ResponseError> {
        for record in chunk {
            let mut is_exist_Merchant= false;
            if self.general_report_on_payments_taxi_company.len() == 0 {
                self.general_report_on_payments_taxi_company.push((
                    // Называние вендора(провайдера)
                    record[collect_indexing.index_provider.unwrap()].to_string(),
                    // Количество транзакций
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    // Общаяя коммиссия
                    record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                ));
                continue;
            }

            for Provider in self.general_report_on_payments_taxi_company.iter_mut() {
                // Считаем сумму, комиссию под каждого вендора
                if Provider.0 == record[collect_indexing.index_provider.unwrap()] {
                    Provider.1 += 1;
                    Provider.2 += record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    Provider.3 += record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    is_exist_Merchant= true;
                    continue;
                }
            }

            if !is_exist_Merchant{
                self.general_report_on_payments_taxi_company.push((
                    // Называние вендора
                    record[collect_indexing.index_provider.unwrap()].to_string(),
                    // Количество транзакций
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    // Общаяя коммиссия
                    record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap(),
                ));
            }
        }

        Ok(())
    }

    pub fn build_general_report_agent(
        &mut self,
        chunk: &Vec<RecordStrings>,
        collect_indexing: &IndexingReport
    ) {
        for record in chunk {
            let mut is_exist_Merchant= false;
            let commission = record[collect_indexing.index_commission.unwrap()].parse::<f64>().unwrap();
            let pay_sys = record[collect_indexing.index_commission_sys.unwrap()].parse::<f64>().unwrap();
            let bank = record[collect_indexing.index_commission_bank.unwrap()].parse::<f64>().unwrap();
            let partner = record[collect_indexing.index_commission_partner.unwrap()].parse::<f64>().unwrap();

            if self.general_report_on_remittance_agent.len() == 0 {
                self.general_report_on_remittance_agent.push((
                    // Название провайдера
                    record[collect_indexing.index_provider.unwrap()].to_string(),
                    // Количество
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()].parse::<f64>().unwrap(),
                    // Комиссия
                    commission,
                    // Вознаграждение COMANYNAME
                    pay_sys + bank,
                    // Вознаграждение Агента
                    partner
                ));
                continue;
            }

            for vendor in self.general_report_on_remittance_agent.iter_mut() {
                // Считаем сумму, комиссию под каждого вендора и его платежную систему
                if vendor.0 == record[collect_indexing.index_provider.unwrap()] {
                    vendor.1 += 1;
                    vendor.2 += record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    vendor.3 += commission;
                    vendor.4 += pay_sys + bank;
                    vendor.5 += partner;
                    is_exist_Merchant= true;
                    continue;
                }
            }

            if !is_exist_Merchant{
                self.general_report_on_remittance_agent.push((
                    // Название провайдера
                    record[collect_indexing.index_provider.unwrap()].to_string(),
                    // Количество
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()].parse::<f64>().unwrap(),
                    // Комиссия
                    commission,
                    // Вознаграждение COMANYNAME
                    pay_sys + bank,
                    // Вознаграждение Агента
                    partner
                ));
            }
        }
    }

    pub fn build_general_report_merchant(
        &mut self,
        chunk: &Vec<RecordStrings>,
        collect_indexing: &IndexingReport,
    ) {
        for record in chunk {
            let mut is_exist_Merchant= false;
            if self.general_report_on_payments_merchant.len() == 0 {
                self.general_report_on_payments_merchant.push((
                    // Называние вендора(провайдера)
                    record[collect_indexing.index_vendor.unwrap()].to_string(),
                    // Платёжная система
                    record[collect_indexing.index_payment_system.unwrap()]
                        .to_string(),
                    // Количество транзакций
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    // Общаяя коммиссия
                    record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                ));
                continue;
            }

            for Provider in self.general_report_on_payments_merchant.iter_mut() {
                // Считаем сумму, комиссию под каждого вендора и его платежную систему
                if Provider.0 == record[collect_indexing.index_vendor.unwrap()]
                    &&
                    Provider.1.to_lowercase() == record[collect_indexing.index_payment_system.unwrap()].to_lowercase() {

                    Provider.2 += 1;
                    Provider.3 += record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    Provider.4 += record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    is_exist_Merchant= true;
                    continue;
                }
            }

            if !is_exist_Merchant{
                self.general_report_on_payments_merchant.push((
                    // Называние вендора
                    record[collect_indexing.index_vendor.unwrap()].to_string(),
                    record[collect_indexing.index_payment_system.unwrap()].to_string(),
                    // Количество транзакций
                    1,
                    // Сумма
                    record[collect_indexing.index_amount.unwrap()]
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    // Общаяя коммиссия
                    record[collect_indexing.index_commission.unwrap()]
                        .parse::<f64>()
                        .unwrap(),
                ));
            }
        }
    }

    pub fn get_min_mount_from_filed_day_in_report(&self) -> u8 {
        let mut min_mount = 0;

        let _ = &self.days_in_report.iter().for_each(|date| {
            let current_mount = date.split("-").collect::<Vec<&str>>()[1]
                .parse::<u8>()
                .unwrap_or(1);
            if min_mount == 0 {
                min_mount = current_mount;
            }
            if current_mount < min_mount {
                min_mount = current_mount;
            }
        });

        min_mount
    }

    pub fn set_refill_amount(&mut self, amount: f64)  {
        self.refill_amount = amount;
    }
}

impl Report {
    pub fn new(report_type: ReportType, provider_id: String) -> Report {
        Report {
            report_type,
            report_organization_name: "".to_string(),
            report_organization_id: provider_id,
            report_items: HashMap::new(),
            is_report_read: false,
            create_at: Utc::now().timestamp(),
        }
    }

    pub fn create_empty_item(&mut self, item_type: ReportItemType, filter: &Filter) {
        let empty_item = ReportItem {
            filter: filter.clone(),
            amount: 0.0,
            days_amount: vec![],
            days_len_transaction: vec![],
            len_transactions: 0,
            commission_by_day: vec![],
            commission: 0.0,
            percent_load: 100.0,
            id_having_chunk: vec![],
            summary_by_Provider: vec![],
            general_report_on_payments_taxi_company: vec![],
            general_report_on_payments_merchant: vec![],
            general_report_on_remittance_agent: vec![],
            refill_amount: 0.0,
            days_in_report: Default::default(),
            all_types_of_commissions: CommissionType {
                commission: 0.0,
                commission_pay_sys: 0.0,
                commission_bank: 0.0,
                commission_payment: 0.0,
                commission_eops: 0.0,
                commission_partner: 0.0,
            },
        };

        self.report_items.insert(item_type, empty_item);
    }

    pub fn set_report(&mut self, report_item_type: ReportItemType, filter: &Filter) {
        self.report_items.insert(report_item_type, ReportItem::new(filter));
    }

    pub fn set_report_read_true(&mut self) {
        self.is_report_read = true;
    }

    pub fn set_provider_id(&mut self, provider_id: String) {
        if !provider_id.is_empty() && self.report_organization_id.is_empty() {
            self.report_organization_id = provider_id;
        }
    }

    pub fn set_Provider_name(&mut self, Provider_name: String) {
        if !Provider_name.is_empty() && self.report_organization_name.is_empty() {
            self.report_organization_name = Provider_name;
        }
    }

    pub fn get_organization_id(&self) -> String {
        let organization_id = &self.report_organization_id;
        organization_id.clone()
    }

    pub fn get_provider_name(&self) -> String { self.report_organization_name.clone() }

    pub fn get_formatted_date(&self) -> DateTime<Local> {
        Local.timestamp_opt(self.create_at, 0).unwrap()
    }

    pub fn get_percent_load_by_report_item_type(&mut self, filter: &Filter) -> f64 {
        match filter.get_type_report_that_generated() {
            None => 0.0,
            Some(item_type) => {
                match self.get_report_item(item_type) {
                    None => 0.0,
                    Some(report_item) => report_item.percent_load
                }
            }
        }
    }

    pub fn get_report_item<'a>(&'a mut self, key: &'a ReportItemType) -> Option<&'a mut ReportItem> {
        self.report_items.get_mut(key)
    }

    pub fn get_report_type(&self) -> &ReportType {
        &self.report_type
    }

    // return 0. c2card report_item and return 1. report_item pay
    /// Возврощает первым аргументом report_item c2card
    /// Вторым аргументом pay
    pub fn get_remittance_and_payments(&mut self) -> (Option<ReportItem>, Option<ReportItem>) {

        let borrow_report_item_remittance = if let Some(report_item) = self.report_items.get(&ReportItemType::Remittance).clone() {
            Some(report_item.clone())
        } else {
            None
        };

        let borrow_report_item_payments = if let Some(report_item) = self.report_items.get(&ReportItemType::Payments).clone() {
            Some(report_item.clone())
        } else {
            None
        };

        (
            borrow_report_item_remittance,
            borrow_report_item_payments
        )
    }

    pub fn get_remittance_and_payments_date(&mut self) -> Option<(String, String)> {
        let days_rem = if let Some(item_report) = self.report_items.get(&ReportItemType::Remittance) {
            let days_in_report = item_report.days_in_report.clone();
            Some(days_in_report.into_iter().collect::<Vec<String>>())
        } else {
            None
        };

        let days_pay = if let Some(item_report) = self.report_items.get(&ReportItemType::Payments) {
            let days_in_report = item_report.days_in_report.clone();
            Some(days_in_report.into_iter().collect::<Vec<String>>())
        } else {
            None
        };

        match days_rem {
            None => {
                match days_pay {
                    Some(mut days_pay) => {
                        days_pay.sort();
                        let first_date = match days_pay.first() {
                            None => None,
                            Some(first_date) => Some(first_date.clone())
                        };

                        let last_date = match days_pay.last() {
                            None => None,
                            Some(last_date) => Some(last_date.clone())
                        };
                        match first_date.is_none() || last_date.is_none() {
                            true => return None,
                            false => Some((first_date.unwrap(), last_date.unwrap()))
                        }
                    }
                    None => None
                }
            }
            Some(mut days_rem) => {
                days_rem.sort();
                let first_date = match days_rem.first() {
                    None => None,
                    Some(first_date) => Some(first_date.clone())
                };

                let last_date = match days_rem.last() {
                    None => None,
                    Some(last_date) => Some(last_date.clone())
                };

                match first_date.is_none() || last_date.is_none() {
                    true => return None,
                    false => Some((first_date.unwrap(), last_date.unwrap()))
                }
            }
        }
    }

    pub fn get_report_Provider(&self) -> String {
        self.report_organization_name.clone()
    }

    pub fn get_all_report_item_keys(&self) -> Vec<&ReportItemType> {
        let mut report_item_type = Vec::new();
        for key in self.report_items.keys() {
            report_item_type.push(key);
        }

        return report_item_type;
    }

    /// Пушим в Share данные для сиаимсимки
    pub fn push_in_share_records_by_chunks(
        &mut self,
        records: Vec<Vec<String>>,
        chunk_num: usize,
        number_of_chunks: usize,
        collect_indexing: IndexingReport,
        type_report: ReportItemType,
        report_type: ReportType,
    ) -> Result<f64, ResponseError> {
        //===================================================================================================\\
        // EN Here we get the date and amount for the day that are in the chunk
        // RU Здесь мы получаем сумму и дни которые доступны в чанке
        let amount_per_day = Share::calculate_amount_commission(
            &records,
            collect_indexing.index_amount.unwrap(),
            &collect_indexing,
        );
        let commission_per_day = Share::calculate_amount_commission(
            &records,
            collect_indexing.index_commission.unwrap(),
            &collect_indexing,
        );
        //===================================================================================================\\

        let ref_report_item= self.get_report_item(&type_report);

        if let None = ref_report_item {
            return Err((4132425, "Не удалось получить часть отчета".to_string()));
        }

        let report_item = ref_report_item.unwrap();

        if let Err(error) = report_item.build_summary_by_Provider(&records, &collect_indexing, &type_report) {
            error!("{}", format!("code: {} message {}", error.0, error.1));
            return Err(error);
        }

        if report_type == ReportType::TaxiCompany || report_type == ReportType::Agent {
            if let Err(error) = report_item.build_general_report_taxi_company(&records, &collect_indexing) {
                error!("{}", format!("code: {} message {}", error.0, error.1));
                return Err(error);
            }
        }

        if report_type == ReportType::Agent && type_report == ReportItemType::Remittance {
            report_item.build_general_report_agent(&records, &collect_indexing);
        }

        if report_type == ReportType::Merchant {
            report_item.build_general_report_merchant(&records, &collect_indexing);
        }

        for (day, _) in amount_per_day.iter() {
            if report_item.days_in_report.get(day.as_str()).is_none() {
                report_item.days_in_report.insert(day.clone());
            }
        }

        report_item.amount += Share::calculate_amount(&records,collect_indexing.index_amount.unwrap_or(8));
        // Считаем все виды комиссии
        report_item.calculate_all_type_commissions(&records, &collect_indexing, &type_report);

        // Считаем комиссию общую комиссию
        report_item.calculate_commission(&records, &collect_indexing);
        // Устанавливаем amount за каждый день
        report_item.set_days_amount(amount_per_day);
        // Устанавливаем commission за каждый день;
        report_item.set_days_commissions(commission_per_day);

        // Тут мы добовляем дни в месяце и их amount
        for day in &report_item.days_in_report {
            let mut exist = false;
            for date in report_item.days_len_transaction.iter() {
                if date.0 == day.to_string() {
                    exist = true;
                    break;
                }
            }
            if !exist {
                report_item.days_len_transaction.push((day.to_string(), 0));
            }
        }

        // Считаем количемство транзакций за каждый день
        Share::count_transaction(
            &records,
            &mut report_item.days_len_transaction,
            &collect_indexing,
        );
        // Устанавливаем общее количество транзакций
        report_item.set_len_transactions(records.len());
        // Добавляем новый id загруженного чанка
        report_item.id_having_chunk.push(chunk_num as u32);
        // Обновляем процент загруженности
        Share::check_percentage_load_report(report_item, number_of_chunks);

        Ok(report_item.percent_load)
    }
}

#[derive(Debug)]
pub struct Reports {
    pub data: TokioRwLock<HashMap<String, ArcMutexWrapper<Report>>>,
}

impl Reports {
    pub fn initial_key(
        &self,
        report_type: &ReportType,
        organization_provider_id: &str,
        from_to: &ReportsDateRange,
        s_m_p: (Vec<Status>, Vec<String>, Vec<Vec<String>>),
        id: String,
    ) -> String {
        let mut statuses = s_m_p.0;
        statuses.sort();
        let mut modes = s_m_p.1;
        modes.sort();
        let mut payments_system = s_m_p.2;
        payments_system.iter_mut().for_each(|system| {
            system.sort();
        });
        payments_system.sort();

        create_file_name(report_type, organization_provider_id, from_to, &statuses, &modes, id, &payments_system)
    }

    pub async fn insert_new_report(&self, key: String, report: ArcMutexWrapper<Report>) {
        self.data.write().await.insert(
            key,
            report
        );
    }

    pub async fn get_report(&self, key: &str) -> Option<Arc<TokioRwLock<Report>>> {
        match self.data.read().await.get(key) {
            None => None,
            Some(report) => Some(Arc::clone(&report.0))
        }
    }

    pub async fn get_keys(&self) -> Vec<String> {
        self.data.read().await.keys().into_iter().map(|key| key.clone()).collect::<Vec<String>>()
    }
}

// @43231
impl Share {
    pub fn new() -> Share {
        Share {
            reports: Reports {
                data: TokioRwLock::new(HashMap::new()),
            },
            generated_now: Arc::new(Mutex::new(0)),
            max_count_record_in_reports: Arc::new(Mutex::new(dotenv!("MAX_NUMBER_OF_REPORTS_IN_SHARE")
                .parse::<u16>()
                .unwrap_or(1000))),
        }
    }

    /// Создает новую запись в share, для орентировки по датам,
    /// Она выглядит примерно так 2023-01-01#2023-01-31
    pub fn create_new_from_to(from: String, to: String) -> String {
        let from_to = from.to_string() + "#" + to.as_str();
        from_to
    }

    /// Перенести функцию в ArcMutexWrapper и сделать её на асинхронной
    pub async fn sort_len_transaction(&self, key: &str) -> Result<(), ResponseError> {
        let mut report_reader = self.reports.data.write().await;
        let report = match report_reader.get_mut(key) {
            Some(report) => Some(report),
            None => None
        };

        if let None = report {
            return Ok(());
        }

        let report_arc = Arc::clone(&report.unwrap().0);
        let mut report = report_arc.write().await;

        match report.get_report_item(&ReportItemType::Payments) {
            Some(item_report) => {
                let _ = item_report.days_len_transaction.sort_by(|a, b| {
                    let date_a = NaiveDate::parse_from_str(a.0.as_str(), "%Y-%m-%d").unwrap();
                    let date_b = NaiveDate::parse_from_str(b.0.as_str(), "%Y-%m-%d").unwrap();
                    date_a.cmp(&date_b)
                });
            },
            None => {}
        };
        match report.get_report_item(&ReportItemType::Remittance) {
            Some(item_report) => {
                let _ = item_report.days_len_transaction.sort_by(|a, b| {
                    let date_a = NaiveDate::parse_from_str(a.0.as_str(), "%Y-%m-%d").unwrap();
                    let date_b = NaiveDate::parse_from_str(b.0.as_str(), "%Y-%m-%d").unwrap();
                    date_a.cmp(&date_b)
                });
            },
            None => {}
        };

        Ok(())
    }

    pub fn is_exist_file_report(&self, file_name: &String, user_id: &str ) -> (bool, Option<String>) {
        let path_to_file = format!("{}/reports/{}/{}.xlsx", dotenv!("REPORTS_DIR"), user_id, file_name);

        if Path::new(&path_to_file).exists() {
            (true, Some(path_to_file))
        } else {
            (false, None)
        }
    }

    pub async fn is_exist_report(&self, key: &String) -> bool {
        if self.reports.data.read().await.get(key).is_some() {
            true
        } else {
            false
        }
    }

    /// Подсчитывает amount в одном чанке который потом мы запишем в share либо прибавим к существующему числу
    pub fn calculate_amount(chunk: &Vec<Vec<String>>, index_amount: usize) -> f64 {
        // Пересчитываем все поля amount
        // amount не был протестирован
        let amount = if chunk.is_empty() {
            0.0
        } else {
            chunk
                .iter()
                .map(|sum| {
                    if sum.get(index_amount).is_none() {
                        0.0
                    } else {
                        sum.get(index_amount).unwrap().parse::<f64>().unwrap_or(0.0)
                    }
                })
                .collect::<Vec<f64>>()
                .iter()
                .sum()
        };

        amount
    }

    /// Считаем amount на каждый день сгенерированного отчета
    pub fn calculate_amount_commission(
        records: &Vec<Vec<String>>,
        index: usize,
        collecting_index: &IndexingReport,
    ) -> Vec<(String, f64)> {
        let mut amounts_for_days: Vec<(String, f64)> = Vec::new();

        let first_date = records.get(0).map_or("".to_string(), |info| {
            return info
                .get(collecting_index.index_date.unwrap())
                .clone()
                .map_or("".to_string(), |rec_data| {
                    rec_data
                        .clone()
                        .split(" ")
                        .collect::<Vec<&str>>()
                        .get(0)
                        .unwrap_or(&&"")
                        .to_string()
                });
        });

        let last_date = records
            .get(records.len() - 1)
            .map_or("".to_string(), |info| {
                return info
                    .get(collecting_index.index_date.unwrap())
                    .clone()
                    .map_or("".to_string(), |rec_data| {
                        rec_data
                            .clone()
                            .split(" ")
                            .collect::<Vec<&str>>()
                            .get(0)
                            .unwrap_or(&&"")
                            .to_string()
                    });
            });

        let (first_year, first_month, first_day) = get_number_from_date(&first_date);
        let (last_year, last_mont, last_day) = get_number_from_date(&last_date);

        let start_date = NaiveDate::from_ymd_opt(first_year, first_month, first_day);
        let end_date = NaiveDate::from_ymd_opt(last_year, last_mont, last_day);

        if let None = start_date {
            error!(
                "Err: code: {} message: {}",
                43214231,
                "Не удалось получить \"start_date\"".to_string()
            );
        }

        if let None = end_date {
            error!(
                "Err: code: {} message: {}",
                43214232,
                "Не удалось получить \"end_date\"".to_string()
            );
            // return Err((43214231, "Не удалось получить \"end_date\"".to_string()))
        }

        let mut current_date = start_date.unwrap();
        let mut period_date = Vec::new();

        while current_date <= end_date.unwrap() {
            period_date.push(current_date);
            current_date += Duration::days(1);
        }

        if &first_date != &"".to_string() && &last_date != &"".to_string() {
            period_date.iter().for_each(|date| {
                let mut date_exist = false;

                for (available_date, _) in amounts_for_days.iter() {
                    if available_date == &date.to_string() {
                        date_exist = true;
                        break;
                    }
                }

                if !date_exist {
                    amounts_for_days.push((date.to_string(), 0.0))
                }
            });

            for record in records {
                for day in amounts_for_days.iter_mut() {
                    if &day.0
                        == &record
                            .get(collecting_index.index_date.unwrap())
                            .unwrap()
                            .split(" ")
                            .collect::<Vec<&str>>()[0]
                            .parse::<String>()
                            .unwrap()
                    {
                        day.1 += record.get(index).unwrap().parse::<f64>().unwrap();
                    }
                }
            }
        }

        amounts_for_days
    }

    pub fn count_transaction(
        records: &Vec<Vec<String>>,
        list_date: &mut Vec<(String, u64)>,
        collecting_index: &IndexingReport,
    ) {
        for date in list_date.iter_mut() {
            for record in records {
                let only_date = record[collecting_index.index_date.unwrap()]
                    .split(" ")
                    .next()
                    .unwrap();

                if date.0 == only_date {
                    date.1 += 1;
                }
            }
        }
    }

    pub fn check_percentage_load_report(
        report: &mut ReportItem,
        number_of_chunks: usize,
    ) -> Option<f64> {
        let percent = (report.id_having_chunk.len() as f64 / number_of_chunks as f64) * 100.0;

        if percent <= 100.0 {
            report.percent_load = percent.round();
            Some(report.percent_load)
        } else {
            None
        }
    }

    pub fn add_generation(&self) {
        *self.generated_now.lock().unwrap() += 1;
    }

    pub fn take_away_generation(&self) {
        *self.generated_now.lock().unwrap() -= 1;
    }

    pub fn get_number_simultaneous_generations(&self) -> u16 {
        *self.generated_now.lock().unwrap()
    }

    pub fn get_max_count_record_in_reports(&self) -> u16 {
        return *self.max_count_record_in_reports.lock().unwrap();
    }

    pub fn set_refill_amount(&mut self, amount: f64, item_report: &mut ReportItem)  {
        item_report.refill_amount = amount;
    }

    pub async fn processing_chunks(
        report: Arc<TokioRwLock<Report>>,
        chunks: ChunksInReport,
        Provider_name: &mut String,
        filter: &Filter,
        collect_indexing: &IndexingReport,
        report_type: &ReportType
    ) -> Result<(), ResponseError> {
        return if !chunks.is_empty() {
            let mut tasks = Vec::new();

            let total_amount_of_chunks: usize = chunks.len();

            let mut report_guard = report.write().await;

            // Тип генерируемого отчета
            let report_item_type = filter.get_type_report_that_generated().unwrap_or(&ReportItemType::Unknown);

            let is_report_exist = report_guard.get_report_item(report_item_type).is_some();

            if !is_report_exist {
                report_guard.set_report(report_item_type.clone(), filter);

                let process_tasks = processing_tasks(
                    chunks.clone(),
                    Arc::clone(&report),
                    &report_item_type,
                    &collect_indexing,
                    Provider_name,
                    report_type.clone(),
                    total_amount_of_chunks,
                ).await;

                for task in process_tasks {
                    tasks.push(task);
                }

                // Вставляем organization_id
                report_guard.set_Provider_name(Provider_name.clone());
            } else {
                let mut report_item_writer = report.write().await;
                let report_item_opt = report_item_writer.get_report_item(report_item_type);
                let report_item = report_item_opt.unwrap();

                for (chunk_index, chunk) in chunks.into_iter().enumerate() {
                    // Продолжаем с того чанка на котором закончили
                    if chunk_index > *report_item.id_having_chunk.last().unwrap_or(&0) as usize {
                        let task = create_task(
                            Arc::clone(&report),
                            chunk_index,
                            chunk,
                            total_amount_of_chunks,
                            collect_indexing,
                            report_item_type,
                            report_type,
                        ).await;
                        tasks.push(task);
                    }
                }
            }

            drop(report_guard);

            // Если же в нашем tasks находятся задачи, значит нам нужно их обработать
            for task in tasks {
                if let Err(error) = task.await {
                    error!("Ошибка при обработке задачи: {:?}", error);
                    continue;
                }
            }

            Ok(())
        } else {
            // Создаем пустышки
            // share.lock().await.reports.get_report(key).unwrap().lock().unwrap().create_empty_item(report_item_type, filter);
            Ok(())
        };
    }

    pub async fn get_processed_report(&self, key: &str) -> Result<Arc<TokioRwLock<Report>>, ResponseError> {
        let report_res = match self.reports.get_report(key).await {
            Some(report) => Ok(report),
            None => Err((1334300, format!("Не удалось найти скалькулированных данных для отчета {}", key)))
        };

        if let Err(error) = report_res {
            return Err(error)
        }

        let report_mutex = report_res.unwrap();

        Ok(report_mutex)
    }
}

/// Возврощает год, месяц, день
pub fn get_number_from_date(date: &str) -> (i32, u32, u32) {
    let parts: Vec<u32> = date
        .split("-")
        .map(|date| date.parse::<u32>().unwrap_or(0))
        .collect();

    let (year, month, day) = match parts.len() {
        3 => (parts[0] as i32, parts[1], parts[2]),
        2 => (0, parts[0], parts[1]),
        _ => (0, 0, 0),
    };

    (year, month, day)
}
