-- templated: __splink__df_concat_with_tf
-- physical: __splink__df_concat_with_tf_b5d9839a8
CREATE TABLE __splink__df_concat_with_tf_b5d9839a8 AS

    select
    __splink__df_concat.*,
    __splink__df_tf_first_name_06740aa58."tf_first_name",
    __splink__df_tf_email_3ef038b03."tf_email",
    __splink__df_tf_surname_e781fc796."tf_surname",
    __splink__df_tf_city_7b3b60b27."tf_city"
    from __splink__df_concat
    left join __splink__df_tf_first_name_06740aa58 on __splink__df_concat."first_name" = __splink__df_tf_first_name_06740aa58."first_name"
left join __splink__df_tf_email_3ef038b03 on __splink__df_concat."email" = __splink__df_tf_email_3ef038b03."email"
left join __splink__df_tf_surname_e781fc796 on __splink__df_concat."surname" = __splink__df_tf_surname_e781fc796."surname"
left join __splink__df_tf_city_7b3b60b27 on __splink__df_concat."city" = __splink__df_tf_city_7b3b60b27."city"
    
