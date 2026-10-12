-- templated: __splink__df_edges_from_predict
-- physical: __splink__df_edges_from_predict_770712cfc
CREATE TABLE __splink__df_edges_from_predict_770712cfc AS
select
    "unique_id_l" as node_id_l,
    "unique_id_r" as node_id_r
    , match_probability
    from __splink__df_predict_249fb94e6
    where match_probability >= 0.95
