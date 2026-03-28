use crate::AppError;

pub fn parse_i64(value: &str, row_number: usize, field_name: &str) -> Result<i64, AppError> {
	if let Ok(parsed) = value.parse::<i64>() {
		return Ok(parsed);
	}

	if let Ok(parsed) = value.parse::<f64>() {
		if parsed.fract() == 0.0 {
			return Ok(parsed as i64);
		}
	}

	Err(AppError::Processing(format!(
		"Invalid {field_name} at row {row_number}: '{value}' is not an integer"
	)))
}
