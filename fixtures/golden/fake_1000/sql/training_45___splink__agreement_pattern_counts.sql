-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_49875c50d
CREATE TABLE __splink__agreement_pattern_counts_49875c50d AS
select
gamma_first_name,
gamma_surname,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_14d55e90c
group by gamma_first_name, gamma_surname, gamma_city, gamma_email
