-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_b2f5a4e1b
CREATE TABLE __splink__df_concat_with_tf_b2f5a4e1b AS

    select
    __splink__df_concat.*,
    __splink__df_tf_surname_c36195692."tf_surname",
    __splink__df_tf_email_c87f87c88."tf_email",
    __splink__df_tf_first_name_9c8f2b270."tf_first_name",
    __splink__df_tf_city_e8e3ab24d."tf_city"
    from __splink__df_concat
    left join __splink__df_tf_surname_c36195692 on __splink__df_concat."surname" = __splink__df_tf_surname_c36195692."surname"
left join __splink__df_tf_email_c87f87c88 on __splink__df_concat."email" = __splink__df_tf_email_c87f87c88."email"
left join __splink__df_tf_first_name_9c8f2b270 on __splink__df_concat."first_name" = __splink__df_tf_first_name_9c8f2b270."first_name"
left join __splink__df_tf_city_e8e3ab24d on __splink__df_concat."city" = __splink__df_tf_city_e8e3ab24d."city"
    
