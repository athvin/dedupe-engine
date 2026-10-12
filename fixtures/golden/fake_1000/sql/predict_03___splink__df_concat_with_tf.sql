-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_34ffb09cf
CREATE TABLE __splink__df_concat_with_tf_34ffb09cf AS

    select
    __splink__df_concat.*,
    __splink__df_tf_surname_dee359c7c."tf_surname",
    __splink__df_tf_email_67662a74e."tf_email",
    __splink__df_tf_first_name_534a7c5b7."tf_first_name",
    __splink__df_tf_city_ce3138ace."tf_city"
    from __splink__df_concat
    left join __splink__df_tf_surname_dee359c7c on __splink__df_concat."surname" = __splink__df_tf_surname_dee359c7c."surname"
left join __splink__df_tf_email_67662a74e on __splink__df_concat."email" = __splink__df_tf_email_67662a74e."email"
left join __splink__df_tf_first_name_534a7c5b7 on __splink__df_concat."first_name" = __splink__df_tf_first_name_534a7c5b7."first_name"
left join __splink__df_tf_city_ce3138ace on __splink__df_concat."city" = __splink__df_tf_city_ce3138ace."city"
    
