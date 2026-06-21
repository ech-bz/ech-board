use std::net::IpAddr;

pub(crate) struct GeoIp {
    reader: maxminddb::Reader<Vec<u8>>,
}

impl GeoIp {
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Result<Self, maxminddb::MaxMindDbError> {
        Ok(Self {
            reader: maxminddb::Reader::from_source(bytes)?,
        })
    }

    pub(crate) fn country_code(&self, ip: IpAddr) -> Option<u32> {
        let result = self.reader.lookup(ip).ok()?;
        let country = result.decode::<maxminddb::geoip2::Country>().ok()??;
        let iso = country.country.iso_code?;
        iso3166::Country::from_alpha2(iso).map(|c| c.id as u32)
    }
}
