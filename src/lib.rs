use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug)]
pub enum ContainerError {
    MissingEntry,
    MissingSpecializedEntry,
    FactoryError { error: Box<dyn Error> }
}

impl Display for ContainerError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ContainerError::MissingEntry => write!(f, "MissingEntry"),
            ContainerError::MissingSpecializedEntry => write!(f, "MissingSpecializedEntry"),
            ContainerError::FactoryError { error: _ } => write!(f, "FactoryError")
        }
    }
}

impl Error for ContainerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ContainerError::MissingEntry => None,
            ContainerError::MissingSpecializedEntry => None,
            ContainerError::FactoryError { error } => Some(error.as_ref())
        }
    }
}

pub type ContainerResult<T> = Result<T, ContainerError>;

struct KnownSpecializationKey {
    specialization_type_id: TypeId,
    type_id: TypeId
}

impl KnownSpecializationKey {
    fn new(type_id: TypeId, specialization_type_id: TypeId) -> Self {
        Self {
            type_id,
            specialization_type_id
        }
    }

    fn new_for_specialization<T, S>() -> Self where
        T : Clone + 'static,
        S : Copy + 'static,
        i32 : From<S>
    {
        let type_id = TypeId::of::<T>();
        let specialization_type_id = TypeId::of::<S>();
        KnownSpecializationKey::new(type_id, specialization_type_id)
    }
}

impl PartialEq for KnownSpecializationKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id &&
            self.specialization_type_id == other.specialization_type_id
    }
}

impl Eq for KnownSpecializationKey { }

impl Hash for KnownSpecializationKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.specialization_type_id.hash(state);
    }
}

impl Clone for KnownSpecializationKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            specialization_type_id: self.specialization_type_id
        }
    }
}

impl Copy for KnownSpecializationKey { }

struct SpecializedEntryKey {
    specialization_value: i32,
    specialization_type_id: TypeId,
    type_id: TypeId
}

impl SpecializedEntryKey {
    fn new(type_id: TypeId, specialization_type_id: TypeId, specialization_value: i32) -> Self {
        SpecializedEntryKey {
            type_id,
            specialization_type_id,
            specialization_value
        }
    }

    fn new_for_specialization<T, S>(specialization: S) -> Self where
        T : Clone + 'static,
        S : Copy + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let specialization_value: i32 = specialization.into();
        let type_id = TypeId::of::<T>();
        let specialization_type_id = TypeId::of::<S>();
        SpecializedEntryKey::new(type_id, specialization_type_id, specialization_value)
    }
}

impl PartialEq for SpecializedEntryKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id &&
            self.specialization_type_id == other.specialization_type_id &&
            self.specialization_value == other.specialization_value
    }
}

impl Eq for SpecializedEntryKey { }

impl Hash for SpecializedEntryKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.specialization_type_id.hash(state);
        self.specialization_value.hash(state);
    }
}

impl Clone for SpecializedEntryKey {
    fn clone(&self) -> Self {
        SpecializedEntryKey {
            type_id: self.type_id,
            specialization_type_id: self.specialization_type_id,
            specialization_value: self.specialization_value
        }
    }
}

impl Copy for SpecializedEntryKey { }

enum ContainerEntry {
    Instance(Arc<dyn Any>),
    Factory(Arc<dyn Fn(&Container) -> ContainerResult<Arc<dyn Any>>>),
    SpecializedFactory(Arc<dyn Fn(&Container) -> ContainerResult<Arc<dyn Any>>>)
}

impl Clone for ContainerEntry {
    fn clone(&self) -> Self {
        match self {
            ContainerEntry::Instance(instance) => {
                ContainerEntry::Instance(instance.clone())
            },
            ContainerEntry::Factory(factory) => {
                ContainerEntry::Factory(factory.clone())
            },
            ContainerEntry::SpecializedFactory(factory) => {
                ContainerEntry::SpecializedFactory(factory.clone())
            }
        }
    }
}

pub struct Container<'container> {
    entries: RefCell<HashMap<TypeId, ContainerEntry>>,
    specialized_entries: RefCell<HashMap<SpecializedEntryKey, ContainerEntry>>,
    specializations: RefCell<HashMap<KnownSpecializationKey, HashSet<i32>>>,
    spooky_ghost: PhantomData<&'container dyn Any>
}

impl<'container> Container<'container> {
    pub fn new() -> Self {
        Self {
            entries: RefCell::new(HashMap::new()),
            specialized_entries: RefCell::new(HashMap::new()),
            specializations: RefCell::new(HashMap::new()),
            spooky_ghost: PhantomData
        }
    }

    pub fn register_instance<T>(&self, instance: T) -> &Self where
        T : Clone + 'static
    {
        let type_id = TypeId::of::<T>();
        self.entries.borrow_mut().insert(type_id, ContainerEntry::Instance(Arc::new(instance)));
        self
    }

    pub fn register_specialized_instance<T, F, S>(&self, specialization: S, instance: T) -> &Self where
        T : Clone + 'static,
        S : Copy + 'static,
        F: Fn(&Container, S) -> T + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let specialized_entry_key = SpecializedEntryKey::new_for_specialization::<T, S>(specialization);
        self.specialized_entries.borrow_mut().insert(specialized_entry_key, ContainerEntry::Instance(Arc::new(instance)));
        self.register_specialization::<T, S>(specialization);
        self
    }

    pub fn register_factory<T, F>(&self, factory: F) -> &Self where
        T : Clone + 'static,
        F : Fn(&Container) -> Result<T, ContainerError> + 'static
    {
        let type_id = TypeId::of::<T>();
        let any_factory = move |container: &Container| -> ContainerResult<Arc<dyn Any>> {
            match factory(container) {
                Ok(new_instance) => Ok(Arc::new(new_instance)),
                Err(err) => Err(err)
            }
        };
        self.entries.borrow_mut().insert(type_id, ContainerEntry::Factory(Arc::new(any_factory)));
        self
    }

    pub fn register_specialized_factory<T, S, F>(&self, specialization: S, factory: F) -> &Self where
        T : Clone + 'static,
        S : Copy + 'static,
        F: Fn(&Container) -> Result<T, ContainerError> + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let specialized_entry_key = SpecializedEntryKey::new_for_specialization::<T, S>(specialization);
        let any_factory = move |container: &Container| -> ContainerResult<Arc<dyn Any>> {
            match factory(container) {
                Ok(new_instance) => Ok(Arc::new(new_instance)),
                Err(err) => Err(err)
            }
        };
        self.specialized_entries.borrow_mut().insert(specialized_entry_key, ContainerEntry::SpecializedFactory(Arc::new(any_factory)));
        self.register_specialization::<T, S>(specialization);
        self
    }

    pub fn default<T>(&self) -> ContainerResult<T> where
        T : Clone + 'static
    {
        let type_id = TypeId::of::<T>();
        let entry = {
            self.entries.borrow().get(&type_id).cloned()
        };

        match entry {
            Some(container_entry) => {
                match container_entry {
                    ContainerEntry::Instance(instance) => {
                        Ok((*instance).downcast_ref::<T>().unwrap().clone())
                    },
                    ContainerEntry::Factory(factory) => {
                        match factory(self) {
                            Ok(new_instance) => {
                                let new_entry = ContainerEntry::Instance(new_instance);
                                {
                                    let mut entries = self.entries.borrow_mut();
                                    entries.insert(type_id, new_entry);
                                }
                                self.default()
                            },
                            Err(err) => Err(err)
                        }
                    },
                    _ => {
                        Err(ContainerError::MissingEntry)
                    }
                }
            },
            None => {
                Err(ContainerError::MissingEntry)
            }
        }
    }

    pub fn specialized<T, S>(&self, specialization: S) -> ContainerResult<T> where
        T : Clone + 'static,
        S : Copy + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let specialized_entry_key = SpecializedEntryKey::new_for_specialization::<T, S>(specialization);

        let specialized_entry = {
            self.specialized_entries.borrow().get(&specialized_entry_key).cloned()
        };

        match specialized_entry {
            Some(container_entry) => {
                match container_entry {
                    ContainerEntry::Instance(instance) => {
                        Ok((*instance).downcast_ref::<T>().unwrap().clone())
                    },
                    ContainerEntry::SpecializedFactory(factory) => {
                        match factory(self) {
                            Ok(new_instance) => {
                                let new_entry = ContainerEntry::Instance(new_instance);
                                {
                                    let mut specialized_entries = self.specialized_entries.borrow_mut();
                            specialized_entries.insert(specialized_entry_key, new_entry);
                                }
                                self.specialized(specialization)
                            },
                            Err(err) => Err(err)
                        }
                    },
                    _ => {
                        Err(ContainerError::MissingSpecializedEntry)
                    }
                }
            },
            None => {
                Err(ContainerError::MissingSpecializedEntry)
            }
        }
    }

    pub fn all_specialized<T, S>(&self) -> ContainerResult<Vec<T>> where
        T : Clone + 'static,
        S : Copy + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let known_specialization_key = KnownSpecializationKey::new_for_specialization::<T, S>();
        let mut specializations = self.specializations.borrow_mut();
        let known_specializations_entry = specializations.entry(known_specialization_key)
            .or_insert_with(|| { HashSet::new() });
        let mut instances = Vec::new();
        for specialization_value in known_specializations_entry.iter() {
            let specialization: S = (*specialization_value).into();
            match self.specialized(specialization) {
                Ok(specialized_instance) => instances.push(specialized_instance),
                Err(err) => return Err(err)
            }
        }
        Ok(instances)
    }

    fn register_specialization<T, S>(&self, specialization: S) where
        T : Clone + 'static,
        S : Copy + 'static,
        i32 : From<S>,
        S : From<i32>
    {
        let known_specialization_key = KnownSpecializationKey::new_for_specialization::<T, S>();
        let mut specializations = self.specializations.borrow_mut();
        let known_specializations_entry = specializations.entry(known_specialization_key)
            .or_insert_with(|| { HashSet::new() });
        let specialization_value: i32 = specialization.into();
        known_specializations_entry.insert(specialization_value);
    }
}
