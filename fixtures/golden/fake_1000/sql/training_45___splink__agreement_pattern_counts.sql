-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_cc55f0595
CREATE TABLE __splink__agreement_pattern_counts_cc55f0595 AS
select
gamma_first_name,
gamma_surname,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_2ae7abac7
group by gamma_first_name, gamma_surname, gamma_city, gamma_email
