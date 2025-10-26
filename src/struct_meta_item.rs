use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct StructMetaItem {
    pub name: String,
    pub src: String,
    pub fields: Vec<StructField>,
}

impl StructMetaItem {
    pub fn new(parsed: structparse::Struct, src: String) -> anyhow::Result<Self> {
        let fields: anyhow::Result<Vec<StructField>> =
            parsed.fields.into_iter().map(try_resolve_field).collect();
        Ok(Self {
            name: parsed.name.to_string(),
            src,
            fields: fields?,
        })
    }
    pub fn fields_with_offsets_mut(&mut self) -> impl Iterator<Item = (usize, &mut StructField)> {
        let mut offset = 0;
        let mut fields = self.fields.iter_mut();
        std::iter::from_fn(move || {
            let field = fields.next()?;
            let ty_size = field.ty.size();
            let item = (offset, &mut *field);
            offset += ty_size;
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
            let prim = match ident {
                "i8" => StructPrimitive::I8,
                "u8" => StructPrimitive::U8,
                "i16" => StructPrimitive::I16,
                "u16" => StructPrimitive::U16,
                "i32" => StructPrimitive::I32,
                "u32" => StructPrimitive::U32,
                "i64" => StructPrimitive::I64,
                "u64" => StructPrimitive::U64,
                "f32" => StructPrimitive::F32,
                "f64" => StructPrimitive::F64,
                _ => anyhow::bail!("Unknown type"),
            };
            Ok(StructTy::Primitive {
                ty: prim,
                endian: Endian::Le,
            })
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

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Endian {
    Le,
    Be,
}

impl Endian {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Le => "le",
            Self::Be => "be",
        }
    }

    pub(crate) fn toggle(&mut self) {
        *self = match self {
            Self::Le => Self::Be,
            Self::Be => Self::Le,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StructTy {
    Primitive { ty: StructPrimitive, endian: Endian },
    Array { item_ty: Box<Self>, len: usize },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StructPrimitive {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl StructPrimitive {
    fn label(&self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::U8 => "u8",
            Self::I16 => "i16",
            Self::U16 => "u16",
            Self::I32 => "i32",
            Self::U32 => "u32",
            Self::I64 => "i64",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

impl StructTy {
    pub fn size(&self) -> usize {
        match self {
            Self::Primitive { ty, .. } => match ty {
                StructPrimitive::I8 | StructPrimitive::U8 => 1,
                StructPrimitive::I16 | StructPrimitive::U16 => 2,
                StructPrimitive::I32 | StructPrimitive::U32 | StructPrimitive::F32 => 4,
                StructPrimitive::I64 | StructPrimitive::U64 | StructPrimitive::F64 => 8,
            },
            Self::Array { item_ty, len } => item_ty.size() * *len,
        }
    }
    pub fn endian_mut(&mut self) -> &mut Endian {
        match self {
            Self::Primitive { endian, .. } => endian,
            Self::Array { item_ty, .. } => item_ty.endian_mut(),
        }
    }
}

impl std::fmt::Display for StructTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive { ty, endian } => {
                let ty = ty.label();
                let endian = endian.label();
                write!(f, "{ty}-{endian}")
            }
            Self::Array { item_ty, len } => {
                write!(f, "[{item_ty}; {len}]")
            }
        }
    }
}
