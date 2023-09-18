pub mod cryptography {
    use chrono::Local;
    use md5::Digest;
    use rand::Rng;
    use crate::r#trait::filter_report::ReportType;
    use crate::r#type::types::ReportsDateRange;

    pub fn generate_hash_key_for_report(
        report_type: &ReportType,
        organization_name: &str,
        from_to: &ReportsDateRange,
        id: String,
        status: String,
        mode: String,
        payments_system: String
    ) -> Digest {
        let concat_date = from_to.iter().map(|(from, to)| format!(
            "{}_{}", from.clone(), to.clone())
        ).collect::<Vec<String>>().join("_");

        let concatenation_report_info = format!(
            "{}_{}_{}_{}_{}_{}_{}",
            organization_name,
            report_type.report_type_to_string(),
            concat_date,
            id,
            status,
            mode,
            payments_system
        );

        md5::compute(concatenation_report_info.as_bytes())
    }

    pub fn get_new_name(list_name: &str) -> String {
        let hash = md5::compute(Local::now().timestamp().to_string());
        let hash_to_string = hash_16x_to_string(hash);

        format!("{list_name}{}", hash_to_string.to_string())
    }

    pub fn generate_rand_hash() -> String {
        let rng_u32: u32 = rand::thread_rng().gen();
        let rng_string = format!("{}_{}", rng_u32, Local::now().timestamp());
        let hash = md5::compute(rng_string);
        hash_16x_to_string(hash)
    }

    pub fn hash_16x_to_string(hash: Digest) -> String {
        hash.0.iter()
            .map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>().concat()
    }
}
