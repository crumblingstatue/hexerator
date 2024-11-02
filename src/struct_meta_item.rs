use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct StructMetaItem {
    pub name: String,
    pub fields: Vec<StructField>,
}

impl StructMetaItem {
    pub fn new(parsed: structparse::Struct) -> anyhow::Result<Self> {
        let fields: anyhow::Result<Vec<StructField>> =
            parsed.fields.into_iter().map(try_resolve_field).collect();
        Ok(Self {
            name: parsed.name.to_string(),
            fields: fields?,
        })
    }
    pub fn fields_with_offsets(&self) -> impl Iterator<Item = (usize, &StructField)> {
        let mut offset = 0;
        let mut fields = self.fields.iter();
        std::iter::from_fn(move || {
            let field = fields.next()?;
            let item = (offset, field);
            offset += field.ty.size();
            Some(item)
        })
    }
}

fn try_resolve_field(field: structparse::Field) -> anyhow::Result<StructField> {
    Ok(StructField {
        name: field.name.to_string(),
        ty: try_resolve_ty(field.ty)?,
    })
}

fn try_resolve_ty(ty: structparse::Ty) -> anyhow::Result<StructTy> {
    match ty {
        structparse::Ty::Ident(ident) => {
            let ty = match ident {
                "i8" => StructTy::I8,
                "u8" => StructTy::U8,
                "i16" => StructTy::I16,
                "u16" => StructTy::U16,
                "i32" => StructTy::I32,
                "u32" => StructTy::U32,
                "i64" => StructTy::I64,
                "u64" => StructTy::U64,
                _ => anyhow::bail!("Unknown type"),
            };
            Ok(ty)
        }
        structparse::Ty::Array(array) => Ok(StructTy::Array {
            item_ty: Box::new(try_resolve_ty(*array.ty)?),
            len: array.len.try_into()?,
        }),
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: StructTy,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StructTy {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Array { item_ty: Box<StructTy>, len: usize },
}

impl StructTy {
    pub fn size(&self) -> usize {
        match self {
            StructTy::I8 | StructTy::U8 => 1,
            StructTy::I16 | StructTy::U16 => 2,
            StructTy::I32 | StructTy::U32 => 4,
            StructTy::I64 | StructTy::U64 => 8,
            StructTy::Array { item_ty, len } => item_ty.size() * *len,
        }
    }
}

impl std::fmt::Display for StructTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            StructTy::I8 => "i8",
            StructTy::U8 => "u8",
            StructTy::I16 => "i16",
            StructTy::U16 => "u16",
            StructTy::I32 => "i32",
            StructTy::U32 => "u32",
            StructTy::I64 => "i64",
            StructTy::U64 => "u64",
            StructTy::Array { item_ty, len } => {
                write!(f, "[{item_ty}; {len}]")?;
                return Ok(());
            }
        };
        f.write_str(label)
    }
}
