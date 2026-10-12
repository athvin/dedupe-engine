-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_99e6b0070
CREATE TABLE __splink__df_concat_with_tf_99e6b0070 AS

    select
    __splink__df_concat.*,
    __splink__df_tf_surname_f7504392e."tf_surname",
    __splink__df_tf_email_69de97b50."tf_email",
    __splink__df_tf_city_f0977da86."tf_city",
    __splink__df_tf_first_name_cf58e3c11."tf_first_name"
    from __splink__df_concat
    left join __splink__df_tf_surname_f7504392e on __splink__df_concat."surname" = __splink__df_tf_surname_f7504392e."surname"
left join __splink__df_tf_email_69de97b50 on __splink__df_concat."email" = __splink__df_tf_email_69de97b50."email"
left join __splink__df_tf_city_f0977da86 on __splink__df_concat."city" = __splink__df_tf_city_f0977da86."city"
left join __splink__df_tf_first_name_cf58e3c11 on __splink__df_concat."first_name" = __splink__df_tf_first_name_cf58e3c11."first_name"
    
