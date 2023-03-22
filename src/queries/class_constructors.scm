(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          ((block_comment)*
          (line_comment)*
          (constructor_declaration
			(modifiers)* @modifier
			(identifier) @identifier
            (formal_parameters)* @parameters
			))*
      )
	)
)