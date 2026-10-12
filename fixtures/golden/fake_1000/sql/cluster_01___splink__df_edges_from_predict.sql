-- templated: __splink__df_edges_from_predict
-- physical: __splink__df_edges_from_predict_a8e17129f
CREATE TABLE __splink__df_edges_from_predict_a8e17129f AS
select
    "unique_id_l" as node_id_l,
    "unique_id_r" as node_id_r
    , match_probability
    from __splink__df_predict_240f945b3
    where match_probability >= 0.95
