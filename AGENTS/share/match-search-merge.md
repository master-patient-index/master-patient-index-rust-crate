### Match

The matching system compares two records:

- Output: Produces a confidence level probability score 0.00-1.00.
- Configurable scoring: Customizable match thresholds and weights

Matching strategies:

- Probabilistic matching: Advanced fuzzy matching algorithms
- Deterministic matching: Rule-based exact matching

Algorithms:

- Jaro-Winkler similarity: 0.00-1.00, case-insensitive, prefix-bonus
- Jaro-Winkler weighted field-by-field: 0.00-1.00, case-insensitive, only fields present in both records contribute
- Haversine distance with sigmoid decay: geo matching
- Soundex phonetic matching: 4-character code, applied as bonus +0.05 if Soundex match and score < 0.95

### Search

- Full-text search across all fields
- Fuzzy search with configurable tolerance
- Phonetic search (Soundex) integrated into name matching
- Advanced query syntax (AND, OR, NOT)
- High-performance indexing with Tantivy
- Search by name, address, and identifier
- Geo-radius search (find within distance of coordinates)
- Automatic index synchronization with database
- Pagination (offset + limit)
- Option to mask sensitive data in search results

### Duplicate detection

- Batch duplicate detection
- Real-time duplicate checking during record registration (returns 409 Conflict)
- Explicit duplicate-check endpoint
- Threshold-based automatic vs manual review
- Similarity scoring algorithms
- Confidence scoring for match quality (certain/probable/possible)
- Configurable matching rules (threshold, max_candidates, auto_merge_threshold)
- Review queue item generation with status tracking (Pending, Confirmed, Rejected, AutoMerged)

### Merge

- Merge confirmed duplicate records
- Auto-merge for high-confidence matches
- Select master record; transfer data from duplicate to master
- Transfers identifiers, fields, attributes, information, etc.
- Adds duplicate's name as "former" alias on master
- Creates link (Replaces) from master to duplicate
- Marks merged records as inactive (soft delete)
- Maintains merge history with transferred data snapshot
- Publishes Merged event
