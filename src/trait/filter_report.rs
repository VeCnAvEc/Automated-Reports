use crate::helper::generate_xlsx::get_status;
use crate::indexing_report_struct::IndexingReport;
use crate::r#type::types::ResponseError;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::Formatter;
use csv::StringRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    /// [Id] Это id файла в user_interface.lo по которому мы собираемся генерировать отчет.
    pub id: u32,
    /// [Status] Фильтрция по статусу транзакции, есть несколько видов трннзакций к примеру [Завершена, Создана, Ошибка]
    pub status: Option<Status>,
    /// [Mode] Фильтрация по моду, существует несколько видов модов, [Боевой, Тестовый]
    pub mode: Option<String>,
    /// [Payments system] фильтрация по платежным системам
    /// к примеру [Uzcard, QIWI Kassa (₽), MIR Pay и т.д]
    /// Этот фильтр предназначен для [Платежи]
    pub payments_system: Option<Vec<String>>,
    /// [Type report that generated] тип файла по которому генерируется отчет к примеру [pay, c2card, c2cCOMANYNAME и т.д]
    type_report_that_generated: Option<ReportItemType>,
    /// [Type of report we depend] Это поля подставляется само, в зависимости от типа файла, есть такие типы как
    /// [pay, pay_f, c2card, c2cCOMANYNAME, c2cplum и т.д]
    pub type_of_report_we_depend: Option<String>,
    /// [Path to file] Путь до файла по которому идет фильтрация
    path_to_file: Option<String>,
}


impl Filter {
    /// Валидатор по фильтрам, взврщает bool.
    /// [True] если по фильтрам все совпадает.
    /// [False] если хотя бы один фильтр не совпал.
    pub fn filter_validation(
        &self,
        record: &StringRecord,
        collect_indexing: &IndexingReport,
        organization_provider_id: &str,
        report_type: &ReportType
    ) -> Result<bool, ResponseError> {
        let mut filter_status = true;

        if self.status.as_ref().is_some() && self.status.as_ref().unwrap_or(&Status::Unknown) != &Status::Unknown {
            if get_status(&self).unwrap().to_string()
                != record[collect_indexing.index_status.unwrap()]
            {
                filter_status = false;
            }
        } else if self.status.as_ref().is_some() && self.status.as_ref().unwrap_or(&Status::Unknown) == &Status::Unknown {
            filter_status = false;
        }

        if !self.mode.as_ref().unwrap_or(&"".to_string()).is_empty() {
            if self.mode.as_ref().is_some()
                && self.mode.as_ref().unwrap() != &record[collect_indexing.index_mode.unwrap()]
            {
                filter_status = false;
            };
        }

        let index_organization_id = match report_type {
            ReportType::Agent => match collect_indexing.index_provider_id {
                Some(index) => Ok(index),
                None => Err((3443242, "index_Provider не был найден".to_string()))
            },
            ReportType::TaxiCompany => match collect_indexing.index_provider_id {
                Some(index) => Ok(index),
                None => Err((3443243, "index_Provider не был найден".to_string()))
            },
            ReportType::Merchant => match collect_indexing.index_merchant_id {
                Some(index) => Ok(index),
                None => Err((3443244, "index_merchant_id не был найден".to_string()))
            },
            ReportType::Unknown => Err((3443245, "Невозможно распознать index так как вы передали не известный отчет".to_string()))
        };

        if let Err(error) = index_organization_id {
            return Err(error);
        }

        let index_organization_id = index_organization_id.unwrap();

        if !organization_provider_id.is_empty() && organization_provider_id != &record[index_organization_id] {
            filter_status = false;
        }

        if self.payments_system.is_some() {
            let systems_p = self.get_filter_payments_system();
            let mut is_necessary_payment_platform = true;

            // Если в фильтре есть платежные системы, если текущий файл является файлом [Платежи]
            if systems_p.is_some() && self.type_report_that_generated.as_ref().unwrap_or(&ReportItemType::Unknown) == &ReportItemType::Payments {
                let field_index_payment_system = &record[collect_indexing.index_payment_system.unwrap()].to_lowercase();
                for platform in systems_p.unwrap() {
                    // Если фильтр платформы совпал с платформой текущего поля.
                    if &platform.to_lowercase().as_str() == &field_index_payment_system {
                        is_necessary_payment_platform = true;
                        break;
                    } else {
                        is_necessary_payment_platform = false;
                    }
                }

                if !is_necessary_payment_platform {
                    filter_status = false;
                }
            }
        }

        return Ok(filter_status);
    }

    pub fn set_to_lowercase_payments_system_field(&mut self) {
        if self.payments_system.is_some() {
            let mut new_payments_systems = Vec::new();
            let systems = self.payments_system.clone().unwrap();

            for p_system in systems.iter() {
                new_payments_systems.push(p_system.to_lowercase())
            }

            self.payments_system = Some(new_payments_systems);
        }
    }

    pub fn set_type_of_report_we_depend(&mut self, type_of_report_we_depend: String) {
        self.type_of_report_we_depend = Some(type_of_report_we_depend);
    }

    pub fn set_path_to_file(&mut self, path_to_file: String) {
        self.path_to_file = Some(path_to_file);
    }

    pub fn set_type_report_that_generated(&mut self) -> Result<(), (i32, String)> {
        match &self.type_of_report_we_depend {
            None => {
                Err((
                    5436574,
                    format!("Не удалось получить тип файла под id {}", &self.id)
                ))
            },
            Some(report_type) => {
                match report_type.as_str() {
                    "pay" | "pay_f" => {
                        self.type_report_that_generated = Some(ReportItemType::Payments);
                        Ok(())
                    },
                    "c2card" | "c2cCOMANYNAME" => {
                        self.type_report_that_generated = Some(ReportItemType::Remittance);
                        Ok(())
                    },
                    _ => Err((543544, "Не известный тип переданного отчета".to_string()))

                }
            }
        }
    }

    pub fn get_path_to_file(&self) -> Result<String, ResponseError> {
        self.path_to_file.clone().map_or_else(
            || Err((4324223, "Путь до файла отсуствует".to_string())),
            |path| Ok(path),
        )
    }

    pub fn get_type_report_that_generated(&self) -> Option<&ReportItemType> {
        self.type_report_that_generated.as_ref()
    }

    pub fn get_filter_payments_system(&self) -> Option<&Vec<String>> {
        self.payments_system.as_ref()
    }
}

#[derive(Serialize, Debug, Clone, Eq, Hash, PartialEq, Deserialize)]
pub enum ReportItemType {
    Remittance,
    Payments,
    Unknown,
    Empty,
    Null,
}

#[derive(Clone, Debug, Serialize, PartialEq, Ord, Eq, PartialOrd)]
pub enum Status {
    Completed,
    Mistake,
    Created,
    Cancel,
    Null,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Copy)]
pub enum ReportType {
    Agent,
    TaxiCompany,
    Merchant,
    Unknown,
}

impl ReportType {
    pub fn report_type_to_string(&self) -> &str {
        match self {
            ReportType::Agent => "agent",
            ReportType::TaxiCompany => "taxi_compony",
            ReportType::Merchant => "merchant",
            ReportType::Unknown => "unknown",
        }
    }
}

struct StatusVisitor;

impl<'de> Visitor<'de> for StatusVisitor {
    type Value = Status;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an enum variant")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (variant, _): (String, _) = de::EnumAccess::variant(data).unwrap();
        match variant.to_lowercase().as_str() {
            "completed" => Ok(Status::Completed),
            "mistake" => Ok(Status::Mistake),
            "created" => Ok(Status::Created),
            "cancel" => Ok(Status::Cancel),
            "null" => Ok(Status::Null),
            _ => Ok(Status::Unknown),
        }
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &["completed", "created", "", "null"];
        deserializer.deserialize_enum("Status", FIELDS, StatusVisitor)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Status::Completed => write!(f, "Completed"),
            Status::Mistake => write!(f, "Mistake"),
            Status::Created => write!(f, "Created"),
            Status::Cancel => write!(f, "Cancel"),
            Status::Null => write!(f, "Null"),
            Status::Unknown => write!(f, "Unknown"),
        }
    }
}

struct ReportTypeVisitor;

impl<'de> Visitor<'de> for ReportTypeVisitor {
    type Value = ReportType;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an enum variant")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (variant, _): (String, _) = de::EnumAccess::variant(data).unwrap();
        match variant.to_lowercase().as_str() {
            "agent" => Ok(ReportType::Agent),
            "merchant" => Ok(ReportType::Merchant),
            "taxicompany" => Ok(ReportType::TaxiCompany),
            _ => Ok(ReportType::Unknown),
        }
    }
}

impl<'de> Deserialize<'de> for ReportType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &["agent", "merchant", "taxicompany"];
        deserializer.deserialize_enum("ReportType", FIELDS, ReportTypeVisitor)
    }
}