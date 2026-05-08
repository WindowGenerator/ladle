use std::sync::Arc;

use arrow::array::{
    Array, BooleanBuilder, Float32Builder, Int32Builder, LargeStringBuilder, RecordBatch,
};
use arrow::datatypes::{DataType, Field, Schema};
use noodles::vcf::variant::record::AlternateBases as _;
use noodles::vcf::variant::record::Ids as _;
use noodles::vcf::variant::record::samples::Sample as VcfSample;
use noodles::vcf::variant::record::samples::series::value::genotype::Phasing;
use noodles::vcf::header::record::value::map::info::Type as InfoType;

pub fn vcf_base_schema() -> Vec<Field> {
    vec![
        Field::new("chrom", DataType::LargeUtf8, false),
        Field::new("pos",   DataType::Int32,     true),
        Field::new("id",    DataType::LargeUtf8, true),
        Field::new("ref",   DataType::LargeUtf8, false),
        Field::new("alt",   DataType::LargeUtf8, true),
        Field::new("qual",  DataType::Float32,   true),
    ]
}

pub fn info_type_to_arrow(ty: &InfoType) -> DataType {
    match ty {
        InfoType::Integer   => DataType::Int32,
        InfoType::Float     => DataType::Float32,
        InfoType::Flag      => DataType::Boolean,
        InfoType::Character => DataType::LargeUtf8,
        InfoType::String    => DataType::LargeUtf8,
    }
}

pub fn build_schema_with_header(header: &noodles::vcf::Header) -> Arc<Schema> {
    let mut fields = vcf_base_schema();

    for (key, info_map) in header.infos() {
        let dtype = info_type_to_arrow(&info_map.ty());
        fields.push(Field::new(format!("INFO_{key}"), dtype, true));
    }

    let n_samples = header.sample_names().len();
    if n_samples > 0 {
        for (key, _fmt_map) in header.formats() {
            fields.push(Field::new(format!("FMT_{key}"), DataType::LargeUtf8, true));
        }
    }

    Arc::new(Schema::new(fields))
}

pub fn build_vcf_batch(records: &[noodles::vcf::Record]) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let mut chroms    = LargeStringBuilder::with_capacity(n, n * 5);
    let mut positions = Int32Builder::with_capacity(n);
    let mut ids       = LargeStringBuilder::with_capacity(n, n * 5);
    let mut refs      = LargeStringBuilder::with_capacity(n, n * 4);
    let mut alts      = LargeStringBuilder::with_capacity(n, n * 4);
    let mut quals     = Float32Builder::with_capacity(n);

    for rec in records {
        chroms.append_value(rec.reference_sequence_name());

        match rec.variant_start() {
            None          => positions.append_null(),
            Some(Ok(pos)) => positions.append_value(pos.get() as i32),
            Some(Err(_))  => positions.append_null(),
        }

        let id_vec: Vec<String> = rec.ids().iter().map(|s| s.to_string()).collect();
        if id_vec.is_empty() { ids.append_null(); } else { ids.append_value(id_vec.join(";")); }

        refs.append_value(rec.reference_bases());

        let alt_str: Vec<String> = rec.alternate_bases().iter()
            .filter_map(|r| r.ok()).map(|s| s.to_string()).collect();
        if alt_str.is_empty() { alts.append_null(); } else { alts.append_value(alt_str.join(",")); }

        match rec.quality_score() {
            None          => quals.append_null(),
            Some(Ok(v))   => quals.append_value(v),
            Some(Err(_))  => quals.append_null(),
        }
    }

    let columns: Vec<Arc<dyn Array>> = vec![
        Arc::new(chroms.finish()),
        Arc::new(positions.finish()),
        Arc::new(ids.finish()),
        Arc::new(refs.finish()),
        Arc::new(alts.finish()),
        Arc::new(quals.finish()),
    ];

    RecordBatch::try_new(Arc::new(Schema::new(vcf_base_schema())), columns)
}

pub fn build_vcf_batch_with_header(
    records: &[noodles::vcf::Record],
    header: &noodles::vcf::Header,
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let schema = build_schema_with_header(header);

    let info_keys: Vec<String> = header.infos().keys().cloned().collect();
    let fmt_keys: Vec<String>  = header.formats().keys().cloned().collect();
    let n_samples = header.sample_names().len();

    let mut chroms    = LargeStringBuilder::with_capacity(n, n * 5);
    let mut positions = Int32Builder::with_capacity(n);
    let mut ids       = LargeStringBuilder::with_capacity(n, n * 5);
    let mut refs      = LargeStringBuilder::with_capacity(n, n * 4);
    let mut alts      = LargeStringBuilder::with_capacity(n, n * 4);
    let mut quals     = Float32Builder::with_capacity(n);

    enum InfoBuilder {
        Int(Int32Builder),
        Float(Float32Builder),
        Bool(BooleanBuilder),
        Str(LargeStringBuilder),
    }

    let mut info_builders: Vec<InfoBuilder> = header.infos().values().map(|m| {
        match m.ty() {
            InfoType::Integer => InfoBuilder::Int(Int32Builder::with_capacity(n)),
            InfoType::Float   => InfoBuilder::Float(Float32Builder::with_capacity(n)),
            InfoType::Flag    => InfoBuilder::Bool(BooleanBuilder::with_capacity(n)),
            _                 => InfoBuilder::Str(LargeStringBuilder::with_capacity(n, n * 8)),
        }
    }).collect();

    let mut fmt_builders: Vec<LargeStringBuilder> = if n_samples > 0 {
        fmt_keys.iter().map(|_| LargeStringBuilder::with_capacity(n, n * n_samples * 4)).collect()
    } else {
        vec![]
    };

    for rec in records {
        chroms.append_value(rec.reference_sequence_name());

        match rec.variant_start() {
            None          => positions.append_null(),
            Some(Ok(pos)) => positions.append_value(pos.get() as i32),
            Some(Err(_))  => positions.append_null(),
        }

        let id_vec: Vec<String> = rec.ids().iter().map(|s| s.to_string()).collect();
        if id_vec.is_empty() { ids.append_null(); } else { ids.append_value(id_vec.join(";")); }

        refs.append_value(rec.reference_bases());

        let alt_str: Vec<String> = rec.alternate_bases().iter()
            .filter_map(|r| r.ok()).map(|s| s.to_string()).collect();
        if alt_str.is_empty() { alts.append_null(); } else { alts.append_value(alt_str.join(",")); }

        match rec.quality_score() {
            None          => quals.append_null(),
            Some(Ok(v))   => quals.append_value(v),
            Some(Err(_))  => quals.append_null(),
        }

        let rec_info = rec.info();
        let mut info_map: std::collections::HashMap<&str, noodles::vcf::variant::record::info::field::Value<'_>> = std::collections::HashMap::new();
        for result in rec_info.iter(header) {
            if let Ok((k, Some(v))) = result {
                info_map.insert(k, v);
            }
        }

        for (idx, key) in info_keys.iter().enumerate() {
            let val = info_map.get(key.as_str());
            match &mut info_builders[idx] {
                InfoBuilder::Int(b) => {
                    use noodles::vcf::variant::record::info::field::value::Array as InfoArray;
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::Integer(v)) => b.append_value(*v),
                        Some(noodles::vcf::variant::record::info::field::Value::Array(InfoArray::Integer(arr))) => {
                            match arr.iter().next().and_then(|r| r.ok()).flatten() {
                                Some(v) => b.append_value(v),
                                None    => b.append_null(),
                            }
                        }
                        _ => b.append_null(),
                    }
                }
                InfoBuilder::Float(b) => {
                    use noodles::vcf::variant::record::info::field::value::Array as InfoArray;
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::Float(v)) => b.append_value(*v),
                        Some(noodles::vcf::variant::record::info::field::Value::Array(InfoArray::Float(arr))) => {
                            match arr.iter().next().and_then(|r| r.ok()).flatten() {
                                Some(v) => b.append_value(v),
                                None    => b.append_null(),
                            }
                        }
                        _ => b.append_null(),
                    }
                }
                InfoBuilder::Bool(b) => {
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::Flag) => b.append_value(true),
                        _ => b.append_value(false),
                    }
                }
                InfoBuilder::Str(b) => {
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::String(s)) => b.append_value(s.as_ref()),
                        Some(noodles::vcf::variant::record::info::field::Value::Character(c)) => {
                            b.append_value(c.to_string());
                        }
                        Some(noodles::vcf::variant::record::info::field::Value::Array(arr)) => {
                            b.append_value(info_array_to_string(arr));
                        }
                        _ => b.append_null(),
                    }
                }
            }
        }

        if n_samples > 0 && !fmt_keys.is_empty() {
            let samples_data = rec.samples();
            let mut fmt_values: Vec<Vec<Option<String>>> = fmt_keys.iter()
                .map(|_| Vec::with_capacity(n_samples))
                .collect();

            for (sample_idx, sample) in samples_data.iter().enumerate() {
                if sample_idx >= n_samples { break; }
                for (fmt_idx, fmt_key) in fmt_keys.iter().enumerate() {
                    let val_str = match VcfSample::get(&sample, header, fmt_key) {
                        Some(Ok(Some(v))) => Some(sample_value_to_string(v)),
                        _ => None,
                    };
                    fmt_values[fmt_idx].push(val_str);
                }
            }

            for (fmt_idx, vals) in fmt_values.iter().enumerate() {
                if vals.is_empty() {
                    fmt_builders[fmt_idx].append_null();
                } else {
                    let joined: Vec<&str> = vals.iter()
                        .map(|v| v.as_deref().unwrap_or("."))
                        .collect();
                    fmt_builders[fmt_idx].append_value(joined.join("\t"));
                }
            }
        }
    }

    let mut columns: Vec<Arc<dyn Array>> = vec![
        Arc::new(chroms.finish()),
        Arc::new(positions.finish()),
        Arc::new(ids.finish()),
        Arc::new(refs.finish()),
        Arc::new(alts.finish()),
        Arc::new(quals.finish()),
    ];

    for b in info_builders {
        columns.push(match b {
            InfoBuilder::Int(mut b)   => Arc::new(b.finish()),
            InfoBuilder::Float(mut b) => Arc::new(b.finish()),
            InfoBuilder::Bool(mut b)  => Arc::new(b.finish()),
            InfoBuilder::Str(mut b)   => Arc::new(b.finish()),
        });
    }

    for mut b in fmt_builders {
        columns.push(Arc::new(b.finish()));
    }

    RecordBatch::try_new(schema, columns)
}

pub fn info_array_to_string(arr: &noodles::vcf::variant::record::info::field::value::Array<'_>) -> String {
    use noodles::vcf::variant::record::info::field::value::Array;
    match arr {
        Array::Integer(vals) => vals.iter().map(|r| match r {
            Ok(Some(v)) => v.to_string(), _ => ".".to_string(),
        }).collect::<Vec<_>>().join(","),
        Array::Float(vals) => vals.iter().map(|r| match r {
            Ok(Some(v)) => v.to_string(), _ => ".".to_string(),
        }).collect::<Vec<_>>().join(","),
        Array::Character(vals) => vals.iter().map(|r| match r {
            Ok(Some(c)) => c.to_string(), _ => ".".to_string(),
        }).collect::<Vec<_>>().join(","),
        Array::String(vals) => vals.iter().map(|r| match r {
            Ok(Some(s)) => s.to_string(), _ => ".".to_string(),
        }).collect::<Vec<_>>().join(","),
    }
}

pub fn sample_value_to_string(v: noodles::vcf::variant::record::samples::series::Value<'_>) -> String {
    use noodles::vcf::variant::record::samples::series::Value;
    use noodles::vcf::variant::record::samples::series::value::Array;
    match v {
        Value::Integer(i)   => i.to_string(),
        Value::Float(f)     => f.to_string(),
        Value::Character(c) => c.to_string(),
        Value::String(s)    => s.to_string(),
        Value::Genotype(gt) => {
            gt.iter().map(|r| match r {
                Ok((allele, phasing)) => {
                    let a = match allele {
                        Some(idx) => idx.to_string(),
                        None => ".".to_string(),
                    };
                    let sep = match phasing {
                        Phasing::Phased   => "|",
                        Phasing::Unphased => "/",
                    };
                    (a, sep)
                }
                Err(_) => (".".to_string(), "/"),
            })
            .scan(true, |first, (a, sep)| {
                let out = if *first { a.clone() } else { format!("{sep}{a}") };
                *first = false;
                Some(out)
            })
            .collect::<String>()
        }
        Value::Array(arr) => match arr {
            Array::Integer(vals)   => vals.iter().map(|r| match r { Ok(Some(v)) => v.to_string(), _ => ".".to_string() }).collect::<Vec<_>>().join(","),
            Array::Float(vals)     => vals.iter().map(|r| match r { Ok(Some(v)) => v.to_string(), _ => ".".to_string() }).collect::<Vec<_>>().join(","),
            Array::Character(vals) => vals.iter().map(|r| match r { Ok(Some(c)) => c.to_string(), _ => ".".to_string() }).collect::<Vec<_>>().join(","),
            Array::String(vals)    => vals.iter().map(|r| match r { Ok(Some(s)) => s.to_string(), _ => ".".to_string() }).collect::<Vec<_>>().join(","),
        },
    }
}
