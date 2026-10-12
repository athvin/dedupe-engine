-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_0a16efec0
CREATE TABLE __splink__df_concat_with_tf_0a16efec0 AS

    select
    __splink__df_concat.*,
    __splink__df_tf_surname_e96e2162c."tf_surname",
    __splink__df_tf_email_b349a64a8."tf_email",
    __splink__df_tf_city_cc332cc14."tf_city",
    __splink__df_tf_first_name_a70dee059."tf_first_name"
    from __splink__df_concat
    left join __splink__df_tf_surname_e96e2162c on __splink__df_concat."surname" = __splink__df_tf_surname_e96e2162c."surname"
left join __splink__df_tf_email_b349a64a8 on __splink__df_concat."email" = __splink__df_tf_email_b349a64a8."email"
left join __splink__df_tf_city_cc332cc14 on __splink__df_concat."city" = __splink__df_tf_city_cc332cc14."city"
left join __splink__df_tf_first_name_a70dee059 on __splink__df_concat."first_name" = __splink__df_tf_first_name_a70dee059."first_name"
    
