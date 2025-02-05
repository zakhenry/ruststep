// Test for deserializing ISO database.p21 structs

use nom::Finish;
use ruststep::ast::{EntityInstance, Name, Parameter};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;

use ruststep::parser;

fn format_example() -> anyhow::Result<String> {
    let step_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/steps/database.p21");
    let step_str = fs::read_to_string(step_file)?;
    Ok(step_str)
}

#[derive(Debug, Clone)]
struct BSU {
    code: String,
    version: String,
}

#[derive(Debug, Clone)]
struct NonDependentPDet {
    description: String,
    property_bsu_id: u64,
    item_name_id: u64,
    mathematical_string_id: u64,
    data_type_id: u64,
    revision: String,
}


#[derive(Debug, Clone)]
struct ItemLabel {
    description: Option<String>,
    short_name: Option<String>,
}

#[derive(Debug, Clone)]
struct MathematicalString(String);

#[derive(Debug, Default)]
struct DictionaryData {
    class_bsus: HashMap<u64, BSU>,
    property_bsus: HashMap<u64, BSU>,
    data_types: HashMap<u64, DataType>,
    non_dependent_p_dets: HashMap<u64, NonDependentPDet>,
    item_labels: HashMap<u64, ItemLabel>,
    mathematical_strings: HashMap<u64, MathematicalString>,
}

#[derive(Debug)]
struct Class {
    bsu: BSU,
    mathematical_string: MathematicalString,
    item_label: ItemLabel,
}

#[derive(Debug)]
struct Property {
    bsu: BSU,
    mathematical_string: MathematicalString,
    item_label: ItemLabel,
    non_dependent_p_det: NonDependentPDet,
    data_type: DataType,
}

#[derive(Debug, Default)]
struct Dictionary {
    classes: Vec<Class>,
    properties: Vec<Property>,
}

#[derive(Debug, Clone)]
enum DataType {
    String { format: String },
    RealMeasure { format: String, unit_id: u64 },
    Integer { format: String },
    Boolean { format: String },
    Unimplemented { id: u64 },
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self) // @todo
    }
}

impl Property {
    fn format_characteristic(&self) -> String {
        format!("\
Code: {}
Version: {}
Revision: {}
Short Name: {}
Symbol: {}
Description: {}
Data Type: {}
", &self.bsu.code,
                &self.bsu.version,
                &self.non_dependent_p_det.revision,
                &self.item_label.short_name.clone().unwrap_or("?".to_string()),
                &self.mathematical_string.0,
                &self.item_label.description.clone().unwrap_or("?".to_string()),
                &self.data_type
        )
    }
}

#[test]
fn get_owned() {
    let step_str = format_example().unwrap();

    let (_, exchange) = parser::exchange::exchange_file(&step_str).finish().unwrap();

    let mut dictionary_data = DictionaryData::default();

    for entity in &exchange.data[0].entities {
        match entity {
            EntityInstance::Simple { id, record } => {

                // println!("#{id} {}", record.name);

                if let Parameter::List(params) = &record.parameter {
                    match record.name.as_str() {
                        "CLASS_BSU" | "PROPERTY_BSU" => {
                            if let [Parameter::String(name), Parameter::String(version)] = &params[0..2] {
                                let bsu = BSU {
                                    code: name.clone(),
                                    version: version.clone(),
                                };

                                match record.name.as_str() {
                                    "CLASS_BSU" => dictionary_data.class_bsus.insert(*id, bsu),
                                    "PROPERTY_BSU" => dictionary_data.property_bsus.insert(*id, bsu),
                                    _ => unreachable!()
                                };
                            }
                        }
                        // #10492=NON_DEPENDENT_P_DET(#10493, #10499, '001', #10494, TEXT('Angle of the chamfer on the head of a tool item measured between the negative z axis and the chamfer'), $, $, $, #10500, (), #13260, $, #10495, $);
                        "NON_DEPENDENT_P_DET" => {
                            if let [
                            Parameter::Ref(Name::Entity(property_bsu)),
                            Parameter::Ref(_dates),
                            Parameter::String(revision),
                            Parameter::Ref(Name::Entity(item_names)),
                            Parameter::Typed { keyword: _, parameter: description_parameter },
                            _,
                            _,
                            _,
                            Parameter::Ref(Name::Entity(mathematical_string_id)),
                            _synonymous_symbols,
                            _referenced_graphic_id,
                            _,
                            Parameter::Ref(Name::Entity(value_id)),
                            ] = &params[0..13] {
                                if let Parameter::String(description) = &**description_parameter {
                                    let ndpd = NonDependentPDet {
                                        description: description.clone(),
                                        property_bsu_id: *property_bsu,
                                        item_name_id: *item_names,
                                        revision: revision.clone(),
                                        mathematical_string_id: *mathematical_string_id,
                                        data_type_id: *value_id,
                                    };

                                    dictionary_data.non_dependent_p_dets.insert(*id, ndpd);
                                }
                            }
                        }
                        // #11630=ITEM_NAMES(LABEL('tool assembly length'), (), LABEL('tooasslen'), $, $);
                        "ITEM_NAMES" => {
                            if let [
                            Parameter::Typed { keyword: _, parameter: label_parameter },
                            _maybe_synonym,
                            Parameter::Typed { keyword: _, parameter: short_name_parameter },
                            _,
                            _
                            ] = &params[0..5] {
                                let description = if let Parameter::String(description) = &**label_parameter {
                                    Some(description.clone())
                                } else { None };

                                let short_name = if let Parameter::String(description) = &**short_name_parameter {
                                    Some(description.clone()).take_if(|s| !s.is_empty())
                                } else { None };

                                let label = ItemLabel { description, short_name };

                                dictionary_data.item_labels.insert(*id, label);
                            }
                        }

                        "MATHEMATICAL_STRING" => {
                            if let [
                            Parameter::String(s),
                            _
                            ] = &params[0..2] {
                                let mathematical_string = MathematicalString(s.clone());

                                dictionary_data.mathematical_strings.insert(*id, mathematical_string);
                            }
                        }

                        "REAL_MEASURE_TYPE" => {
                            if let [
                            Parameter::String(format),
                            Parameter::Ref(Name::Entity(value_id))
                            ] = &params[0..2] {
                                let measure = DataType::RealMeasure { format: format.clone(), unit_id: *value_id };

                                dictionary_data.data_types.insert(*id, measure);
                            }
                        }

                        "INT_TYPE" => {
                            if let [
                            Parameter::String(format),
                            ] = &params[0..1] {
                                let measure = DataType::Integer { format: format.clone() };

                                dictionary_data.data_types.insert(*id, measure);
                            }
                        }

                        "BOOLEAN_TYPE" => {
                            if let [
                            Parameter::String(format),
                            ] = &params[0..1] {
                                let measure = DataType::Boolean { format: format.clone() };

                                dictionary_data.data_types.insert(*id, measure);
                            }
                        }

                        //                                                                                                                                      v this is the argument that lists the applicable PROPERTY_BSU references
                        // #2159=ITEM_CLASS(#2160, #3597, '002', #2161, TEXT('Family of items designed for use mainly in drilling operations'), $, $, $, #2154, (#1922,#782,#794,#788,#726,#1214,#1973,#1294,#327,#324,#385,#2370,#3353,#777,#344,#2542,#2532,#859,#4886,#2878,#846,#1256,#1262,#1280,#190,#1288,#429,#422,#283,#310 ,#2449,#10096,#10273,#10667,#10675,#11008,#12767,#12754,#12761), (), $, (), (), $);
                        "ITEM_CLASS" => {

                            // @todo parse this message, add references to data dict so each
                            // property can reference the class they are directly applied to
                            // also extend to model the hierarchy as a tree

                        }

                        other => {
                            // println!("unhandled record {}", other);
                        }
                    }
                }
            }
            EntityInstance::Complex { .. } => unreachable!()
        }
    }

    let mut dictionary = Dictionary::default();

    // dbg!(&dictionary_data);

    for (_, non_dependent_p_det) in &dictionary_data.non_dependent_p_dets {
        let property = Property {
            bsu: dictionary_data.property_bsus[&non_dependent_p_det.property_bsu_id].clone(),
            // mathematical_string: dictionary_data.mathematical_strings[&non_dependent_p_det.],
            item_label: dictionary_data.item_labels[&non_dependent_p_det.item_name_id].clone(),
            mathematical_string: dictionary_data.mathematical_strings[&non_dependent_p_det.mathematical_string_id].clone(),
            data_type: dictionary_data.data_types.get(&non_dependent_p_det.data_type_id).unwrap_or(&DataType::Unimplemented { id: non_dependent_p_det.data_type_id.clone() }).clone(),
            non_dependent_p_det: non_dependent_p_det.clone(),
        };

        dictionary.properties.push(property);
    }

    for property in &dictionary.properties {
        println!("{}", property.format_characteristic());
    }
}
