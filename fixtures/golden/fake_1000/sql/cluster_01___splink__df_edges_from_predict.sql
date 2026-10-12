-- templated: __splink__df_edges_from_predict
-- physical: __splink__df_edges_from_predict_42ae78b1e
CREATE TABLE __splink__df_edges_from_predict_42ae78b1e AS
select
    "unique_id_l" as node_id_l,
    "unique_id_r" as node_id_r
    , match_probability
    from __splink__df_predict_a02553e32
    where match_probability >= 0.95
