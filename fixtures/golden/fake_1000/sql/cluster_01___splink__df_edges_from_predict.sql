-- templated: __splink__df_edges_from_predict
-- physical: __splink__df_edges_from_predict_24b0fbcde
CREATE TABLE __splink__df_edges_from_predict_24b0fbcde AS
select
    "unique_id_l" as node_id_l,
    "unique_id_r" as node_id_r
    , match_probability
    from __splink__df_predict_559d520e3
    where match_probability >= 0.95
