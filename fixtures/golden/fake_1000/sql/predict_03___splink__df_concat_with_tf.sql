-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_162a6808c
CREATE TABLE __splink__df_concat_with_tf_162a6808c AS

    select
    __splink__df_concat.*,
    __splink__df_tf_first_name_a591eebe3."tf_first_name",
    __splink__df_tf_email_abfc8347c."tf_email",
    __splink__df_tf_city_28c728c14."tf_city",
    __splink__df_tf_surname_ee74a7353."tf_surname"
    from __splink__df_concat
    left join __splink__df_tf_first_name_a591eebe3 on __splink__df_concat."first_name" = __splink__df_tf_first_name_a591eebe3."first_name"
left join __splink__df_tf_email_abfc8347c on __splink__df_concat."email" = __splink__df_tf_email_abfc8347c."email"
left join __splink__df_tf_city_28c728c14 on __splink__df_concat."city" = __splink__df_tf_city_28c728c14."city"
left join __splink__df_tf_surname_ee74a7353 on __splink__df_concat."surname" = __splink__df_tf_surname_ee74a7353."surname"
    
