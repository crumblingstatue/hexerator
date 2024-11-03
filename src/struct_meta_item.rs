use {
    crate::meta::value_type::{
        EndianedPrimitive as _, I16Be, I16Le, I32Be, I32Le, I64Be, I64Le, U16Be, U16Le, U32Be,
        U32Le, U64Be, U64Le, I8, U8,
    },
    serde::{Deserialize, Serialize},
};

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
            let ty = match ident {
                "i8" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S8,
                    signed: true,
                    endian: Endian::Le,
                },
                "u8" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S8,
                    signed: false,
                    endian: Endian::Le,
                },
                "i16" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S16,
                    signed: true,
                    endian: Endian::Le,
                },
                "u16" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S16,
                    signed: false,
                    endian: Endian::Le,
                },
                "i32" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S32,
                    signed: true,
                    endian: Endian::Le,
                },
                "u32" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S32,
                    signed: false,
                    endian: Endian::Le,
                },
                "i64" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S64,
                    signed: true,
                    endian: Endian::Le,
                },
                "u64" => StructTy::IntegerPrimitive {
                    size: IPrimSize::S64,
                    signed: false,
                    endian: Endian::Le,
                },
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
pub enum Endian {
    Le,
    Be,
}

impl Endian {
    pub fn label(&self) -> &'static str {
        match self {
            Endian::Le => "le",
            Endian::Be => "be",
        }
    }

    pub(crate) fn toggle(&mut self) {
        *self = match self {
            Endian::Le => Endian::Be,
            Endian::Be => Endian::Le,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum IPrimSize {
    S8,
    S16,
    S32,
    S64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StructTy {
    IntegerPrimitive {
        size: IPrimSize,
        signed: bool,
        endian: Endian,
    },

    Array {
        item_ty: Box<StructTy>,
        len: usize,
    },
}

impl StructTy {
    pub fn size(&self) -> usize {
        match self {
            Self::IntegerPrimitive { size, .. } => match size {
                IPrimSize::S8 => 1,
                IPrimSize::S16 => 2,
                IPrimSize::S32 => 4,
                IPrimSize::S64 => 8,
            },
            Self::Array { item_ty, len } => item_ty.size() * *len,
        }
    }
    pub fn read_usize(&self, data: &[u8]) -> Option<usize> {
        match self {
            StructTy::IntegerPrimitive {
                size,
                signed,
                endian,
            } => {
                macro_rules! from_byte_slice {
                    ($t:ty) => {
                        <$t>::from_byte_slice(&data[..self.size()]).and_then(|i| i.try_into().ok())
                    };
                }
                match (size, signed, endian) {
                    (IPrimSize::S8, true, Endian::Le) => from_byte_slice!(I8),
                    (IPrimSize::S8, true, Endian::Be) => from_byte_slice!(I8),
                    (IPrimSize::S8, false, Endian::Le) => from_byte_slice!(U8),
                    (IPrimSize::S8, false, Endian::Be) => from_byte_slice!(U8),
                    (IPrimSize::S16, true, Endian::Le) => from_byte_slice!(I16Le),
                    (IPrimSize::S16, true, Endian::Be) => from_byte_slice!(I16Be),
                    (IPrimSize::S16, false, Endian::Le) => from_byte_slice!(U16Le),
                    (IPrimSize::S16, false, Endian::Be) => from_byte_slice!(U16Be),
                    (IPrimSize::S32, true, Endian::Le) => from_byte_slice!(I32Le),
                    (IPrimSize::S32, true, Endian::Be) => from_byte_slice!(I32Be),
                    (IPrimSize::S32, false, Endian::Le) => from_byte_slice!(U32Le),
                    (IPrimSize::S32, false, Endian::Be) => from_byte_slice!(U32Be),
                    (IPrimSize::S64, true, Endian::Le) => from_byte_slice!(I64Le),
                    (IPrimSize::S64, true, Endian::Be) => from_byte_slice!(I64Be),
                    (IPrimSize::S64, false, Endian::Le) => from_byte_slice!(U64Le),
                    (IPrimSize::S64, false, Endian::Be) => from_byte_slice!(U64Be),
                }
            }
            StructTy::Array { .. } => None,
        }
    }
    pub fn endian_mut(&mut self) -> &mut Endian {
        match self {
            StructTy::IntegerPrimitive { endian, .. } => endian,
            StructTy::Array { item_ty, .. } => item_ty.endian_mut(),
        }
    }
}

impl std::fmt::Display for StructTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructTy::IntegerPrimitive { signed, endian, .. } => {
                let sign = if *signed { "i" } else { "u" };
                let size = self.size() * 8;
                let endian = endian.label();
                write!(f, "{sign}{size}-{endian}")
            }
            StructTy::Array { item_ty, len } => {
                write!(f, "[{item_ty}; {len}]")
            }
        }
    }
}
