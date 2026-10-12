-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_8e40406ee
CREATE TABLE __splink__df_concat_with_tf_8e40406ee AS

    select
    __splink__df_concat.*,
    __splink__df_tf_email_7d3d5a737."tf_email",
    __splink__df_tf_city_45fe4d3d3."tf_city",
    __splink__df_tf_surname_508c09cdf."tf_surname",
    __splink__df_tf_first_name_fba8c2dad."tf_first_name"
    from __splink__df_concat
    left join __splink__df_tf_email_7d3d5a737 on __splink__df_concat."email" = __splink__df_tf_email_7d3d5a737."email"
left join __splink__df_tf_city_45fe4d3d3 on __splink__df_concat."city" = __splink__df_tf_city_45fe4d3d3."city"
left join __splink__df_tf_surname_508c09cdf on __splink__df_concat."surname" = __splink__df_tf_surname_508c09cdf."surname"
left join __splink__df_tf_first_name_fba8c2dad on __splink__df_concat."first_name" = __splink__df_tf_first_name_fba8c2dad."first_name"
    
