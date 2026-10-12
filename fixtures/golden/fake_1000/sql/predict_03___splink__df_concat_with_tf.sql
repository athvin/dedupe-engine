-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_97082f10d
CREATE TABLE __splink__df_concat_with_tf_97082f10d AS

    select
    __splink__df_concat.*,
    __splink__df_tf_surname_88fcd0e79."tf_surname",
    __splink__df_tf_first_name_542f89bfa."tf_first_name",
    __splink__df_tf_city_96a1bcbb1."tf_city",
    __splink__df_tf_email_a41bbfa20."tf_email"
    from __splink__df_concat
    left join __splink__df_tf_surname_88fcd0e79 on __splink__df_concat."surname" = __splink__df_tf_surname_88fcd0e79."surname"
left join __splink__df_tf_first_name_542f89bfa on __splink__df_concat."first_name" = __splink__df_tf_first_name_542f89bfa."first_name"
left join __splink__df_tf_city_96a1bcbb1 on __splink__df_concat."city" = __splink__df_tf_city_96a1bcbb1."city"
left join __splink__df_tf_email_a41bbfa20 on __splink__df_concat."email" = __splink__df_tf_email_a41bbfa20."email"
    
