use rust_container::{Container, ContainerResult};
use std::sync::Arc;

// Example traits + structs
trait Fruit {
    fn is_organic(&self) -> bool;
    fn name(&self) -> &str;
}

struct Banana<'a> {
    is_organic: bool,
    name: &'a str
}

impl<'a> Banana<'a> {
    fn new(is_organic: bool) -> Self {
        Self {
            is_organic,
            name: "banana"
        }
    }
}

impl<'a> Fruit for Banana<'a> {
    fn is_organic(&self) -> bool {
        self.is_organic
    }

    fn name(&self) -> &str {
        self.name
    }
}

trait Meat {
    fn is_organic(&self) -> bool;
    fn name(&self) -> &str;
}

struct Chicken<'a> {
    is_organic: bool,
    name: &'a str
}

impl<'a> Chicken<'a> {
    fn new(is_organic: bool) -> Self {
        Self {
            is_organic,
            name: "chicken"
        }
    }
}

impl<'a> Meat for Chicken<'a> {
    fn is_organic(&self) -> bool {
        self.is_organic
    }

    fn name(&self) -> &str {
        self.name
    }
}

trait GroceryStore {
    fn print_inventory(&self);
}

struct WholeFoods {
    fruit: Arc<dyn Fruit>,
    meat: Arc<dyn Meat>
}

impl WholeFoods {
    fn new(fruit: Arc<dyn Fruit>, meat: Arc<dyn Meat>) -> Self {
        Self {
            fruit,
            meat
        }
    }
}

impl GroceryStore for WholeFoods {
    fn print_inventory(&self) {
        println!("We've got a {}.  Is it organic? {}", self.fruit.name(), self.fruit.is_organic());
        println!("We've also got a {}.  Is it organic? {}", self.meat.name(), self.meat.is_organic());
    }
}

struct KMart { }

impl KMart {
    fn new() -> Self {
        Self { }
    }
}

impl GroceryStore for KMart {
    fn print_inventory(&self) {
        println!("We're closed");
    }
}

struct BasicThing {
    id: i32
}

struct BasicThingWithLifetime<'a> {
    name: &'a str
}

#[derive(Clone, Copy)]
enum GroceryStoreType {
    WholeFoods,
    Kmart
}

impl From<GroceryStoreType> for i32 {
    fn from(enum_value: GroceryStoreType) -> i32 {
        enum_value as i32
    }
}

impl From<i32> for GroceryStoreType {
    fn from(value: i32) -> GroceryStoreType {
        match value {
            0 => GroceryStoreType::WholeFoods,
            1 => GroceryStoreType::Kmart,
            _ => panic!("nope")
        }
    }
}

fn main() {
    let container = Box::new(Container::new());

    container
        .register_factory(|_container: &Container| -> ContainerResult<Arc<Banana>> { Ok(Arc::new(Banana::new(false))) })
        .register_factory(|_container: &Container| -> ContainerResult<Arc<dyn Fruit>> { Ok(Arc::new(Banana::new(true))) })
        .register_factory(|_container: &Container| -> ContainerResult<Arc<Chicken>> { Ok(Arc::new(Chicken::new(true))) })
        .register_factory(|_container: &Container| -> ContainerResult<Arc<dyn Meat>> { Ok(Arc::new(Chicken::new(false))) })
        .register_factory(|container: &Container| -> ContainerResult<Arc<dyn GroceryStore>> { Ok(Arc::new(WholeFoods::new(container.default()?, container.default()?))) })
        .register_factory(|_container: &Container| -> ContainerResult<Arc<BasicThing>> { Ok(Arc::new(BasicThing { id: 42 })) })
        .register_factory(|_container: &Container| -> ContainerResult<Arc<BasicThingWithLifetime>> { Ok(Arc::new(BasicThingWithLifetime { name: "foobar" })) })
        .register_specialized_factory(GroceryStoreType::WholeFoods, |container: &Container| -> ContainerResult<Arc<dyn GroceryStore>> { Ok(Arc::new(WholeFoods::new(container.default()?, container.default()?))) })
        .register_specialized_factory(GroceryStoreType::Kmart, |_container: &Container| -> ContainerResult<Arc<dyn GroceryStore>> { Ok(Arc::new(KMart::new())) });

    let grocery_store: Arc<dyn GroceryStore> = container.default().unwrap();
    grocery_store.print_inventory();

    let basic_thing: Arc<BasicThing> = container.default().unwrap();
    println!("basic_thing id = {}", basic_thing.id);

    let basic_thing_2: Arc<BasicThingWithLifetime> = container.default().unwrap();
    println!("basic_thing_2 name = {}", basic_thing_2.name);

    let banana: Arc<Banana> = container.default().unwrap();
    println!("banana is organic? {}", banana.is_organic());

    let chicken: Arc<Chicken> = container.default().unwrap();
    println!("chicken is organic? {}", chicken.is_organic());

    let the_same_chicken: Arc<Chicken> = container.default().unwrap();
    println!("the_same_chicken is organic? {}", the_same_chicken.is_organic());

    let specialized_whole_foods: Arc<dyn GroceryStore> = container.specialized(GroceryStoreType::WholeFoods).unwrap();
    specialized_whole_foods.print_inventory();

    let specialized_kmart: Arc<dyn GroceryStore> = container.specialized(GroceryStoreType::Kmart).unwrap();
    specialized_kmart.print_inventory();

    let all_grocery_stores: Vec<Arc<dyn GroceryStore>> = container.all_specialized::<Arc<dyn GroceryStore>, GroceryStoreType>().unwrap();
    for (_i, grocery_store) in all_grocery_stores.iter().enumerate() {
        grocery_store.print_inventory();
    }
}
