#[doc(hidden)]
pub use defile::defile;
#[doc = "Apply a `macro_rules!` macro using an `#[apply(macro_name!)]` attribute (provided by `macro_rules_attribute` crate)"]
pub use macro_rules_attribute::apply;
#[doc(hidden)]
pub use paste::paste;

pub mod prelude {
    pub use crate::apply;
    pub use crate::lowboy_record;
    pub use crate::Related;
}

/// A marker to designate a field as being a related model.
pub struct Related<T>(T);

/// Generate record boilerplate for a model.
///
/// # Example
///
/// ```
/// use diesel::prelude::*;
/// # use diesel::sqlite::SqliteConnection;
/// # use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
/// use diesel_async::RunQueryDsl;
/// use lowboy_record::prelude::*;
/// # // Connection type in Lowboy.
/// # type Connection = SyncConnectionWrapper<SqliteConnection>;
///
/// // Normally this is generated by diesel.
/// pub mod schema {
///     # use diesel::table;
///     table! {
///         user (id) {
///             id -> Integer,
///             name -> Text,
///             avatar -> Nullable<Text>,
///         }
///     }
///
///     table! {
///         post (id) {
///             id -> Integer,
///             user_id -> Integer,
///             content -> Text,
///         }
///     }
///
///     table! {
///         comment (id) {
///             id -> Integer,
///             user_id -> Integer,
///             post_id -> Integer,
///             content -> Text,
///         }
///     }
/// }
///
/// # fn main() {
/// lowboy_record! {
///     #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable)]
///     #[diesel(table_name = crate::schema::user)]
///     pub struct User {
///         id: i32,
///         name: String,
///         avatar: Option<String>,
///         posts: Related<Vec<Post>>,
///     }
/// }
///
/// lowboy_record! {
///     #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
///     #[diesel(table_name = crate::schema::post)]
///     #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
///     pub struct Post {
///         id: i32,
///         user: Related<User>,
///         content: String,
///     }
/// }
///
/// // Using the #[apply(macro_name!)] attribute to avoid unnecessary indentation.
/// #[apply(lowboy_record!)]
/// #[derive(Debug, Default, Queryable, Identifiable, Selectable, Insertable, Associations)]
/// #[diesel(table_name = crate::schema::comment)]
/// #[diesel(belongs_to(UserRecord, foreign_key = user_id))]
/// #[diesel(belongs_to(PostRecord, foreign_key = post_id))]
/// pub struct Comment {
///     id: i32,
///     user: Related<User>,
///     post: Related<Post>,
///     content: String,
/// }
/// # }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! lowboy_record {
    // Main entrypoint.
    (
        $(#[$attr:meta])*
        $pub:vis struct $model:ident {
            $($fields:tt)*
        }
    ) => {
        // ModelRecord
        // NewModelRecord
        internal_record!($(#[$attr])* $pub $model ($($fields)*));
        // Model
        internal_model!($pub $model ($($fields)*));
        // impl Model
        internal_impl!($model ($($fields)*));
    };
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! internal_record {
    // Done, generate struct.
    (@record
        ()
        -> { $(#[$attr:meta])* $pub:vis $model:ident $(($field:ident : $type:ty))* }
        [$(($from:ident : $from_type:ty))*]
        [$(($from_related: ident: $from_related_model:ty))*]
    ) => {
        paste! {
            // ModelRecord
            $(#[$attr])*
            #[doc = "A `" $model "` record"]
            $pub struct [<$model Record>] {
                $($field : $type ,)*
            }

            // impl From<Model> for <ModelRecord>
            #[doc = "Convert from a `" $model "` model into `" [<$model Record>] "`"]
            impl From<$model> for [<$model Record>] {
                fn from(value: $model) -> Self {
                $(
                    let $from_related = value.[<$from_related_model:lower>].id;
                )*

                    Self {
                        $($from : value.$from ,)*
                        $($from_related ,)*
                    }
                }
            }
        }

        internal_new_record!($pub $model ($($field : $type ,)*));
    };

    // Strip out vec relation fields. These fields are "virtual" and used for one-to-many relations.
    (@record
        ($field:ident : Related<Vec<$type:ty>> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [$($from:tt)*]
        [$($from_related:tt)*]
    ) => {
        paste! {
            internal_record!(@record ($($($rest)*)?) -> { $($output)* } [$($from)*] [$($from_related)*]);
        }
    };

    // Replace relation fields with foreign key.
    (@record
        ($field:ident : Related<$type:ty> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [$($from:tt)*]
        [$($from_related:tt)*]
    ) => {
        paste! {
            internal_record!(@record ($($($rest)*)?) -> { $($output)* ([<$field _id>] : i32) } [$($from)*] [$($from_related)* ([<$field _id>] : $type)]);
        }
    };

    // Iterate over struct fields.
    (@record
        // Remove the first field/type from the list of Model fields to process into ModelRecord
        // fields.
        ($field:ident : $type:ty $(, $($rest:tt)*)?)
        // Accumulator of ModelRecord output (attrs, visibility, model name, (record fields)).
        -> { $($output:tt)* }
        // Accumulator of non-related fields to copy 1-to-1 from Model to ModelRecord.
        [$($from:tt)*]
        // Accumulator of related fields to copy ids from related Model to ModelRecord.
        [$($from_related:tt)*]
    ) => {
        internal_record!(@record ($($($rest)*)?) -> { $($output)* ($field : $type) } [$($from)* ($field : $type)] [$($from_related)*]);
    };

    // Entrypoint.
    ($(#[$attr:meta])* $pub:vis $model:ident ($($rest:tt)*)) => {
        internal_record!(@record ($($rest)*) -> { $(#[$attr])* $pub $model } [] []);
    };
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
#[allow(clippy::crate_in_macro_def)]
macro_rules! internal_new_record {
    // Done, generate struct and generate new_record associated function for model.
    (@new_record
        ()
        -> { $pub:vis $model:ident $(($field:ident : $type:ty))* }
        [ $(($option:ident : $option_type:ty))* ]
    ) => {
        paste! {
            // NewModelRecord
            #[derive(Clone, Debug, Default, diesel::Insertable)]
            #[diesel(table_name = crate::schema::[<$model:lower>])]
            #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
            $pub struct [<New $model Record>]<'a> {
                $($field : $type ,)*
                $($option : $option_type ,)*
            }

            // impl NewModelRecord
            impl<'a> [<New $model Record>]<'a> {
                // NewModelRecord::new
                #[doc = "Create a new `" [<New $model Record>] "` object"]
                pub fn new($($field : $type ,)*) -> [<New $model Record>]<'a> {
                    Self {
                        $($field ,)*
                        $($option : None ,)*
                    }
                }

            $(
                // NewRecord::with_$option
                #[doc = "Add the optional `" $option "` field to the `" [<New $model Record>] "` object"]
                 pub fn [<with_ $option>](self, $option : $option_type) -> Self {
                    Self {
                        $option,
                        ..self
                    }
                 }
            )*

                // NewModelRecord::create
                #[doc = "Create a new `" [<$model:lower>] "` in the database"]
                pub async fn create(&self, conn: &mut Connection) -> QueryResult<[<$model Record>]> {
                    diesel::insert_into(crate::schema::[<$model:lower>]::table)
                        .values(self)
                        .returning(crate::schema::[<$model:lower>]::table::all_columns())
                        .get_result(conn)
                        .await
                }
            }

            // impl Model
            impl $model {
                // Model::new_record
                #[doc = "Create a new `" [<New $model Record>] "` object"]
                pub fn new_record<'a>($($field : $type ,)*) -> [<New $model Record>]<'a> {
                    [<New $model Record>]::new($($field ,)*)
                }
            }
        }
    };

    // @TODO handle other owned types.

    // Convert Option<String> fields to Option<&'a str>, and put them in the optionial accumulator.
    (@new_record
        ($field:ident : Option<String> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($optional:tt)* ]
    ) => {
        defile! {
            internal_new_record!(@@new_record ($($(@$rest)*)?) -> { $($output)* } [ $($optional)* ($field : Option<&'a str>) ]);
        }
    };

    // Put optional fields in a separate optional accumulator.
    (@new_record
        ($field:ident : Option<$type:ty> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($optional:tt)* ]
    ) => {
        defile! {
            internal_new_record!(@@new_record ($($(@$rest)*)?) -> { $($output)* } [ $($optional)* ($field : Option<$type>) ]);
        }
    };

    // Convert String fields to &'a str.
    (@new_record
        ($field:ident : String $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($optional:tt)* ]
    ) => {
        defile! {
            internal_new_record!(@@new_record ($($(@$rest)*)?) -> { $($output)* ($field : &'a str) } [ $($optional)* ]);
        }
    };

    // Remove id field.
    (@new_record
        (id : $type:ty $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($optional:tt)* ]
    ) => {
        defile! {
            internal_new_record!(@@new_record ($($(@$rest)*)?) -> { $($output)* } [ $($optional)* ]);
        }
    };

    // Iterate over struct fields.
    (@new_record
        // Remove the first field/type from the list of Model fields to process into NewModelRecord
        // fields.
        ($field:ident : $type:ty $(, $($rest:tt)*)?)
        // Accumulator of NewModelRecord output (attrs, visibility, model name, (record fields)).
        -> { $($output:tt)* }
        // Accumulator of optional NewModelRecord fields.
        [ $($optional:tt)* ]
    ) => {
        defile! {
            internal_new_record!(@@new_record ($($(@$rest)*)?) -> { $($output)* ($field : $type) } [ $($optional)* ]);
        }
    };

    // Entrypoint.
    ($pub:vis $model:ident ($($rest:tt)*)) => {
        internal_new_record!(@new_record ($($rest)*) -> { $pub $model } []);
    };
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! internal_model {
    // Done, generate struct.
    (@model
        ()
        -> { $pub:vis $model:ident $(($field:ident : $type:ty))* }
    ) => {
        paste! {
            // Model
            #[derive(Debug, Clone)]
            #[doc = "A `" $model "` model"]
            $pub struct $model {
                $($field : $type ,)*
            }
        }
    };

    // Strip out relation marker.
    (@model
        ($field:ident : Related<$type:ty> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
    ) => {
        internal_model!(@model ($($($rest)*)?) -> { $($output)* ($field : $type) });
    };

    // Iterate over struct fields.
    (@model
        // Remove the first field/type from the list of Model fields to process into Model fields.
        ($field:ident : $type:ty $(, $($rest:tt)*)?)
        // Accumulator of Model output (attrs, visibility, model name, (model fields)).
        -> { $($output:tt)* }
    ) => {
        internal_model!(@model ($($($rest)*)?) -> { $($output)* ($field : $type) });
    };

    // Entrypoint.
    ($pub:vis $model:ident ($($rest:tt)*)) => {
        internal_model!(@model ($($rest)*) -> { $pub $model });
    };
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
#[allow(clippy::crate_in_macro_def)]
macro_rules! internal_impl {
    // Done, generate Model impl.
    (@impl
        ()
        -> { $model:ident $(($field:ident : $type:ty))* }
        [ $(($key:ident ; $foreign_key:ident : $foreign_model:ty))* ]
        [ $(($many:ident : $many_model:ty))* ]
    ) => {
        // impl Model
        impl $model {
            paste! {
                // Model::from_record
                #[doc = "Create a `" $model "` object from a `" [<$model Record>] "`"]
                #[doc = "This will also load child models, excluding one-to-many children."]
                pub async fn from_record(record: &[<$model Record>], conn: &mut Connection) -> QueryResult<Self> {
                    $(
                        let $key: [<$foreign_model Record>] = crate::schema::[<$foreign_model:lower>]::table
                            .find(record.$foreign_key)
                            .first(conn)
                            .await?;
                        let $key = $foreign_model::from_record(&$key, conn).await?;
                    )*

                    Ok($model {
                        $($key ,)*
                        $(
                            $field : record.$field.clone(),
                        )*
                        $($many : Vec::new() ,)*
                    })
                }

                // Model::from_records
                #[doc = "Create `" $model "` objects from a vec of `" [<$model Record>] "`"]
                #[doc = "This will also load child models, excluding one-to-many children."]
                pub async fn from_records<'a>(
                    records: impl IntoIterator<Item = &'a [<$model Record>]>,
                    conn: &'a mut Connection,
                ) -> QueryResult<Vec<Self>> {
                    let mut models = Vec::new();
                    for record in records.into_iter() {
                        models.push(Self::from_record(record, conn).await?);
                    }

                    Ok(models)
                }

            $(
                // Model::with_$many
                #[doc = "Load `" $many "` models into the `" [<$model>] "` object"]
                pub async fn [<with_ $many>](self, conn: &mut Connection) -> QueryResult<Self> {
                    let record: [<$model Record>] = self.clone().into();
                    let records: Vec<[<$many_model Record>]> = [<$many_model Record>]::belonging_to(&record)
                        .select(crate::schema::[<$many_model:lower>]::table::all_columns())
                        .load(conn)
                        .await?;

                    let mut $many = Vec::new();
                    for record in &records {
                        $many.push($many_model::from_record(record, conn).await?);
                    }

                    Ok(Self {
                        $many,
                        ..self
                    })
                }
            )*

            }
        }
    };

    // Put vec relation fields in a separate one-to-many accumulator.
    (@impl
        ($field:ident : Related<Vec<$type:ty>> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($relations:tt)* ]
        [ $($many:tt)* ]
    ) => {
        paste! {
            internal_impl!(@impl ($($($rest)*)?) -> { $($output)* } [ $($relations)* ] [ $($many)* ($field : $type) ]);
        }
    };

    // Put relation fields in a separate accumulator.
    (@impl
        ($field:ident : Related<$type:ty> $(, $($rest:tt)*)?)
        -> { $($output:tt)* }
        [ $($relations:tt)* ]
        [ $($many:tt)* ]
    ) => {
        paste! {
            internal_impl!(@impl ($($($rest)*)?) -> { $($output)* } [ $($relations)* ($field ; [<$field _id>] : $type) ] [ $($many)* ]);
        }
    };

    // Iterate over struct fields.
    (@impl
        // Remove the first field/type from the list of Model fields to process into Model fields.
        ($field:ident : $type:ty $(, $($rest:tt)*)?)
        // Accumulator of Impl output (model name, (model fields)).
        -> { $($output:tt)* }
        // Accumulator of model children.
        [ $($relations:tt)* ]
        // Accumulator of model child collections.
        [ $($many:tt)* ]
    ) => {
        internal_impl!(@impl ($($($rest)*)?) -> { $($output)* ($field : $type) } [ $($relations)* ] [ $($many)* ]);
    };

    // Entrypoint.
    ($model:ident ($($rest:tt)*)) => {
        internal_impl!(@impl ($($rest)*) -> { $model } [] []);
    };
}
