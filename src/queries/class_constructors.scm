(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          ((block_comment)*
          (line_comment)*
          (constructor_declaration
			(modifiers)* @modifier
      (marker_annotation)* @annotation
			(identifier) @identifier
            (formal_parameters)* @parameters
            (throws)* @throws
			))*
      )
	)
)